"""Real SIGTERM regression for the composed worker, against disposable PostgreSQL.

Usage: python3 worker_binary_test.py /path/to/oteryn-portal
PORTAL_TEST_DATABASE_URL selects the database. PORTAL_TEST_PSQL optionally selects
an existing psql wrapper; its default connection must select that same database.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit
import uuid


def test_locked_topup_sigterm_completes_within_shutdown_budget(binary):
    started = time.monotonic()
    schema = "fix07_" + uuid.uuid4().hex
    command = [os.environ.get("PORTAL_TEST_PSQL", "psql"), "-X", "-At", "-v", "ON_ERROR_STOP=1"]
    env = os.environ.copy()
    env["PGDATABASE"] = env["PORTAL_TEST_DATABASE_URL"]

    def sql(text):
        result = subprocess.run(command, input=f"SET search_path={schema}; {text}", env=env,
                                text=True, capture_output=True, check=True)
        return result.stdout.strip().splitlines()[-1]

    sql(f"CREATE SCHEMA {schema}; SELECT 1;")
    url = urlsplit(env["PORTAL_TEST_DATABASE_URL"])
    options = dict(parse_qsl(url.query))
    options["options"] = f"-csearch_path={schema}"
    env["OTERYN_DATABASE_URL"] = urlunsplit(url._replace(query=urlencode(options)))
    env.update(OTERYN_ENV="testing", OTERYN_PAYMENTS_ENABLED="true", OTERYN_WALLET_ENABLED="true")
    worker = lock = None
    try:
        subprocess.run([binary, "migrate"], env=env, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        account, intent = sql("SELECT uuidv7(), uuidv7();").split("|")
        payload = json.dumps(dict(intent_id=intent, account_id=account, amount=250, currency="PLN"))
        sql(f"""
            INSERT INTO accounts(id,email,email_verified_at,password_hash,status,locale)
              VALUES('{account}','sigterm@example.invalid',now(),'unused','active','en');
            INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state)
              VALUES('{intent}','{account}','test','{intent}',250,'PLN','succeeded');
            INSERT INTO store_web_topups(operation_id,account_id,package_key,coins,minor_units,currency,payment_intent_id)
              VALUES(uuidv7(),'{account}','fixed',1234,250,'PLN','{intent}');
            INSERT INTO outbox(kind,payload,idempotency_key) VALUES('wallet.top_up','{payload}','sigterm');
            SELECT 1;
        """)
        lock = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, env=env, text=True, bufsize=1)
        lock.stdin.write(f"SET search_path={schema}; BEGIN; SELECT id FROM payment_intents WHERE id='{intent}' FOR UPDATE;\n")
        lock.stdin.flush()
        while lock.stdout.readline().strip() != intent:
            assert lock.poll() is None, "lock holder exited"
        worker = subprocess.Popen([binary, "worker"], env=env,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        deadline = time.monotonic() + 15
        while sql(f"SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT account_id,amount,currency,state FROM payment_intents%' AND query LIKE '%FOR UPDATE%';") == "0":
            assert worker.poll() is None, "worker exited before delivery"
            assert time.monotonic() < deadline, "worker did not reach intent lock"
            time.sleep(0.02)
        signalled = time.monotonic()
        worker.send_signal(signal.SIGTERM)
        worker.wait(timeout=11)
        elapsed = time.monotonic() - signalled
        assert worker.returncode == 0, f"worker exit code {worker.returncode}"
        assert elapsed < 10.75, f"shutdown exceeded 10s drain budget plus scheduling tolerance: {elapsed:.3f}s"
        assert sql("SELECT count(*) FROM wallet_entries;") == "0"
        assert sql("SELECT count(*) FROM outbox WHERE kind='wallet.top_up' AND state IN ('pending','running');") == "1"
        print(f"PASS test_locked_topup_sigterm_completes_within_shutdown_budget signal_to_exit={elapsed:.3f}s duration={time.monotonic()-started:.3f}s")
    finally:
        if worker and worker.poll() is None:
            worker.kill()
            worker.wait()
        if lock:
            lock.stdin.write("ROLLBACK;\n\\q\n")
            lock.stdin.flush()
            lock.wait(timeout=5)
        sql(f"DROP SCHEMA {schema} CASCADE; SELECT 1;")


if __name__ == "__main__":
    test_locked_topup_sigterm_completes_within_shutdown_budget(str(Path(sys.argv[1]).resolve()))
