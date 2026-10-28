<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\DB;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('store_purchase_requests', function (Blueprint $table): void {
            $table->uuid('purchase_request_id')->primary();
            $table->uuid('account_id'); // Unknown accounts must also have durable refusal answers.
            $table->uuid('character_id');
            $table->uuid('world_id');
            $table->char('catalog_revision', 64); // Stale/unknown revisions can be refused; no FK.
            $table->string('offer_key', 64);
            $table->string('currency', 20);
            $table->bigInteger('unit_price');
            $table->integer('quantity');
            $table->text('request_bytes');
            $table->char('request_digest', 64);
            $table->string('result', 32);
            $table->text('answer_bytes');
            $table->timestampTz('decided_at');
        });
        Schema::create('store_orders', function (Blueprint $table): void {
            $table->uuid('order_id')->primary();
            $table->uuid('purchase_request_id')->unique();
            $table->foreign('purchase_request_id')->references('purchase_request_id')->on('store_purchase_requests')->restrictOnDelete();
            $table->foreignId('identity_id')->constrained('identities')->restrictOnDelete();
            $table->string('wallet_idempotency_key', 120)->unique();
            $table->string('currency', 20);
            $table->bigInteger('total_price');
            $table->timestampTz('created_at');
        });
        Schema::create('store_order_lines', function (Blueprint $table): void {
            $table->uuid('line_id')->primary();
            $table->uuid('order_id')->unique(); // v1: one bought offer per order.
            $table->foreign('order_id')->references('order_id')->on('store_orders')->restrictOnDelete();
            $table->char('catalog_revision', 64);
            $table->string('offer_key', 64);
            $table->foreign(['catalog_revision', 'offer_key'])->references(['catalog_revision', 'offer_key'])->on('store_offers')->restrictOnDelete();
            $table->integer('quantity');
            $table->bigInteger('unit_price');
        });
        Schema::create('store_fulfilment_units', function (Blueprint $table): void {
            $table->uuid('entitlement_id')->primary();
            $table->uuid('line_id');
            $table->foreign('line_id')->references('line_id')->on('store_order_lines')->restrictOnDelete();
            $table->integer('unit_index');
            $table->unique(['line_id', 'unit_index']);
            $table->uuid('account_id');
            $table->string('product_id', 64);
            $table->decimal('product_version', 20, 0);
            $table->string('unit_kind', 16);
            $table->string('grant_key', 64);
            $table->decimal('grant_revision', 20, 0);
            $table->integer('quantity');
            $table->string('state', 32);
            $table->timestampTz('created_at');
        });
        Schema::create('store_operations', function (Blueprint $table): void {
            $table->bigIncrements('source_revision');
            $table->uuid('operation_id')->unique();
            $table->string('issuer_authority', 64);
            $table->unique(['issuer_authority', 'operation_id']);
            $table->string('operation_kind', 20);
            $table->uuid('entitlement_id');
            $table->foreign('entitlement_id')->references('entitlement_id')->on('store_fulfilment_units')->restrictOnDelete();
            $table->uuid('delivery_operation_id')->nullable();
            $table->string('reversal_reason', 24)->nullable();
            $table->text('intent_bytes');
            $table->char('binding_digest', 64);
            $table->string('state', 16);
            $table->timestampTz('created_at');
            $table->index(['state', 'source_revision', 'operation_id']);
        });
        Schema::table('store_operations', function (Blueprint $table): void {
            $table->foreign('delivery_operation_id')->references('operation_id')->on('store_operations')->restrictOnDelete();
        });
        Schema::create('store_receipts', function (Blueprint $table): void {
            $table->id();
            $table->string('issuer_authority', 64);
            $table->uuid('operation_id');
            $table->foreign(['issuer_authority', 'operation_id'])->references(['issuer_authority', 'operation_id'])->on('store_operations')->restrictOnDelete();
            $table->string('operation_kind', 20);
            $table->char('binding_digest', 64);
            $table->string('outcome', 12);
            $table->string('result_code', 40);
            $table->text('receipt_bytes');
            $table->char('receipt_digest', 64);
            $table->decimal('recovery_generation', 20, 0);
            $table->decimal('decided_at', 20, 0);
            $table->uuid('superseded_by_restore_event_id')->nullable();
            $table->timestampTz('received_at');
            $table->index(['operation_id', 'id']);
        });
        // Durable per-notice reopen membership, including carried outstanding operations. No restore handler here.
        Schema::create('store_reconcile_states', function (Blueprint $table): void {
            $table->id();
            $table->uuid('restore_event_id');
            $table->uuid('operation_id');
            $table->foreign('operation_id')->references('operation_id')->on('store_operations')->restrictOnDelete();
            $table->unsignedBigInteger('superseded_receipt_id')->nullable();
            $table->foreign('superseded_receipt_id')->references('id')->on('store_receipts')->restrictOnDelete();
            $table->decimal('restored_generation', 20, 0);
            $table->decimal('new_recovery_generation', 20, 0);
            $table->unsignedBigInteger('resolved_receipt_id')->nullable();
            $table->foreign('resolved_receipt_id')->references('id')->on('store_receipts')->restrictOnDelete();
            $table->unique(['restore_event_id', 'operation_id']);
        });

        if (DB::getDriverName() === 'pgsql') {
            $this->constraints();
        }
    }

    private function constraints(): void
    {
        $kinds = "('DELIVER','CLAIM','REVOKE_UNCLAIMED','RECLAIM','FLAG')";
        DB::statement("ALTER TABLE store_purchase_requests ADD CHECK (result IN ('ACCEPTED','INSUFFICIENT_BALANCE','STALE_QUOTE','OFFER_UNAVAILABLE','ACCOUNT_RESTRICTED','PURCHASE_DISABLED') AND currency IN ('COINS','TRANSFERABLE_COINS') AND unit_price > 0 AND quantity BETWEEN 1 AND 100 AND octet_length(request_bytes) BETWEEN 1 AND 1024 AND octet_length(answer_bytes) BETWEEN 1 AND 2048 AND request_digest = encode(sha256(convert_to(request_bytes, 'UTF8')), 'hex'))");
        DB::statement("ALTER TABLE store_orders ADD CHECK (currency = 'COINS' AND total_price > 0 AND wallet_idempotency_key = 'store-purchase:' || purchase_request_id::text)");
        DB::statement('ALTER TABLE store_order_lines ADD CHECK (quantity BETWEEN 1 AND 100 AND unit_price > 0)');
        DB::statement("ALTER TABLE store_fulfilment_units ADD CHECK (unit_kind IN ('PREMIUM_TIME','COSMETIC','ITEM') AND quantity BETWEEN 1 AND 65535 AND unit_index BETWEEN 0 AND 7 AND state IN ('ENTITLEMENT_ACTIVE','DELIVERY_PENDING','DELIVERED','CLAIMED','RECONCILING','REVERSAL_REQUESTED','REVERSED','REVERSAL_PARTIAL','MANUAL_REVIEW'))");
        DB::statement("ALTER TABLE store_operations ADD CHECK (issuer_authority = 'OTERYN_PLATFORM_STORE_AUTHORITY' AND operation_kind IN {$kinds} AND state IN ('PENDING','REOPENED','COMMITTED','REJECTED') AND (reversal_reason IS NULL OR reversal_reason IN ('REFUND','CHARGEBACK','SUPPORT_CORRECTION')) AND ((operation_kind IN ('REVOKE_UNCLAIMED','RECLAIM','FLAG')) = (reversal_reason IS NOT NULL)) AND ((operation_kind = 'DELIVER') = (delivery_operation_id IS NULL)) AND octet_length(intent_bytes) BETWEEN 1 AND 4096 AND binding_digest = encode(sha256(convert_to(intent_bytes, 'UTF8')), 'hex'))");
        DB::statement("CREATE UNIQUE INDEX store_one_deliver_per_unit ON store_operations (entitlement_id) WHERE operation_kind = 'DELIVER'");
        DB::statement("CREATE UNIQUE INDEX store_one_claim_per_unit ON store_operations (entitlement_id) WHERE operation_kind = 'CLAIM'");
        DB::statement('CREATE UNIQUE INDEX store_current_receipt ON store_receipts (issuer_authority, operation_id) WHERE superseded_by_restore_event_id IS NULL');
        DB::statement("ALTER TABLE store_receipts ADD CHECK (operation_kind IN {$kinds} AND outcome IN ('COMMITTED','REJECTED') AND binding_digest ~ '^[0-9a-f]{64}$' AND octet_length(receipt_bytes) BETWEEN 1 AND 1024 AND receipt_digest = encode(sha256(convert_to(receipt_bytes, 'UTF8')), 'hex'))");
        DB::statement("ALTER TABLE store_receipts ADD CHECK ((outcome = 'COMMITTED' AND ((result_code = 'STORE_OK' AND operation_kind IN ('DELIVER','CLAIM','REVOKE_UNCLAIMED','FLAG')) OR (result_code IN ('STORE_RECLAIMED','STORE_RECLAIM_SHORTFALL') AND operation_kind = 'RECLAIM'))) OR (outcome = 'REJECTED' AND (result_code IN ('STORE_INTENT_INVALID','STORE_OPERATION_CONFLICT','STORE_UNSUPPORTED') OR (result_code = 'STORE_ACCOUNT_NOT_FOUND' AND operation_kind = 'DELIVER') OR (result_code = 'STORE_GRANT_UNKNOWN' AND operation_kind IN ('DELIVER','CLAIM')) OR (result_code = 'STORE_LINE_REVOKED' AND operation_kind = 'CLAIM') OR (result_code = 'STORE_LINE_ALREADY_CLAIMED' AND operation_kind = 'REVOKE_UNCLAIMED') OR (result_code = 'STORE_LINE_NOT_FOUND' AND operation_kind IN ('CLAIM','REVOKE_UNCLAIMED','RECLAIM','FLAG')))))");
        foreach (['store_fulfilment_units' => ['product_version', 'grant_revision'], 'store_receipts' => ['recovery_generation', 'decided_at'],
            'store_reconcile_states' => ['restored_generation', 'new_recovery_generation']] as $table => $columns) {
            foreach ($columns as $column) {
                DB::statement("ALTER TABLE {$table} ADD CHECK ({$column} BETWEEN 1 AND 18446744073709551615)");
            }
        }
        DB::statement('ALTER TABLE store_reconcile_states ADD CHECK (new_recovery_generation > restored_generation)');
        DB::unprepared(<<<'SQL'
            CREATE OR REPLACE FUNCTION store_preserve_intent() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'Store intent cannot be deleted'; END IF;
                IF (to_jsonb(NEW) - 'state') IS DISTINCT FROM (to_jsonb(OLD) - 'state') THEN
                    RAISE EXCEPTION 'Store intent binding is immutable';
                END IF;
                RETURN NEW;
            END;
            $$;
            CREATE TRIGGER store_intent_immutable BEFORE UPDATE OR DELETE ON store_operations
            FOR EACH ROW EXECUTE FUNCTION store_preserve_intent();
            CREATE OR REPLACE FUNCTION store_preserve_purchase_answer() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN RAISE EXCEPTION 'Stored Store purchase answers are immutable'; END;
            $$;
            CREATE TRIGGER store_purchase_answer_immutable BEFORE UPDATE OR DELETE ON store_purchase_requests
            FOR EACH ROW EXECUTE FUNCTION store_preserve_purchase_answer();
            CREATE OR REPLACE FUNCTION store_preserve_receipt() RETURNS trigger LANGUAGE plpgsql AS $$
            BEGIN
                IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'Store receipt cannot be deleted'; END IF;
                IF OLD.superseded_by_restore_event_id IS NOT NULL OR NEW.superseded_by_restore_event_id IS NULL
                    OR (to_jsonb(NEW) - 'superseded_by_restore_event_id') IS DISTINCT FROM
                       (to_jsonb(OLD) - 'superseded_by_restore_event_id') THEN
                    RAISE EXCEPTION 'Store receipt bytes are immutable';
                END IF;
                RETURN NEW;
            END;
            $$;
            CREATE TRIGGER store_receipt_immutable BEFORE UPDATE OR DELETE ON store_receipts
            FOR EACH ROW EXECUTE FUNCTION store_preserve_receipt();
            SQL);
    }

    public function down(): void
    {
        foreach (['store_reconcile_states', 'store_receipts', 'store_operations', 'store_fulfilment_units',
            'store_order_lines', 'store_orders', 'store_purchase_requests'] as $table) {
            Schema::dropIfExists($table);
        }
        if (DB::getDriverName() === 'pgsql') {
            DB::statement('DROP FUNCTION IF EXISTS store_preserve_intent()');
            DB::statement('DROP FUNCTION IF EXISTS store_preserve_purchase_answer()');
            DB::statement('DROP FUNCTION IF EXISTS store_preserve_receipt()');
        }
    }
};
