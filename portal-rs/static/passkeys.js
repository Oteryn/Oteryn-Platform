/* Browser transport only; all WebAuthn verification happens on the server. */
(() => {
    "use strict";
    const root = document.getElementById("passkeys");
    if (!root) return;
    const status = document.getElementById("passkey-status");
    const decode = value => Uint8Array.from(atob(value.replace(/-/g, "+").replace(/_/g, "/")), c => c.charCodeAt(0));
    const encode = value => btoa(String.fromCharCode(...new Uint8Array(value))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
    async function post(path, body) {
        const response = await fetch(root.dataset.base + path, {
            method: "POST", credentials: "same-origin", cache: "no-store",
            headers: {"Content-Type": "application/json", "X-CSRF-Token": root.dataset.csrf},
            body: JSON.stringify(body)
        });
        if (!response.ok) throw new Error("Request refused");
        return response.json();
    }
    root.addEventListener("click", async event => {
        const button = event.target.closest("button");
        if (!button || button.disabled) return;
        const buttons = root.querySelectorAll("button");
        buttons.forEach(b => { b.disabled = true; });
        try {
            const password = document.getElementById("passkey-password");
            if (button.dataset.remove) {
                await post("/" + button.dataset.remove + "/remove", {password: password.value});
            } else {
                const kind = button.dataset.passkey;
                const start = await post("/" + kind + "/start", kind === "register" ? {password: password.value} : {});
                const options = start.options;
                options.publicKey.challenge = decode(options.publicKey.challenge);
                if (options.publicKey.user) options.publicKey.user.id = decode(options.publicKey.user.id);
                for (const key of ["excludeCredentials", "allowCredentials"]) {
                    (options.publicKey[key] || []).forEach(c => { c.id = decode(c.id); });
                }
                if (kind === "authenticate") document.getElementById("passkey-username").focus();
                const credential = kind === "register" ? await navigator.credentials.create(options) : await navigator.credentials.get(options);
                const response = {};
                for (const key of ["clientDataJSON", "attestationObject", "authenticatorData", "signature", "userHandle"]) {
                    if (credential.response[key]) response[key] = encode(credential.response[key]);
                }
                if (credential.response.getTransports) response.transports = credential.response.getTransports();
                await post("/" + kind + "/finish", {token: start.token, credential: {
                    id: credential.id, rawId: encode(credential.rawId), type: credential.type,
                    response, extensions: credential.getClientExtensionResults()
                }});
            }
            status.textContent = status.dataset.success;
            window.location.reload();
        } catch (_) {
            status.textContent = status.dataset.error;
            buttons.forEach(b => { b.disabled = false; });
        }
    });
})();
