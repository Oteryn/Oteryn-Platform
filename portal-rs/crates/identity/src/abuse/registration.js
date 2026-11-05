"use strict";
// Work is computed locally. No password or password hash is sent to a third party.
const form = document.querySelector('form[action$="/register"]');
if (form) {
    let busy = false;
    form.addEventListener("submit", async (event) => {
        if (form.elements.work_token.value && form.elements.work_nonce.value) return;
        event.preventDefault();
        if (busy) return;
        busy = true;
        form.setAttribute("aria-busy", "true");
        try {
            const response = await fetch("/register/abuse/challenge", {
                method: "POST",
                credentials: "same-origin",
                headers: { "x-csrf-token": form.elements._csrf.value }
            });
            if (!response.ok) throw new Error("unavailable");
            const challenge = await response.json();
            if (!/^[0-9A-F]{64}$/.test(challenge.token) || challenge.bits !== 16) throw new Error("invalid");
            const encoder = new TextEncoder();
            const deadline = Date.now() + 240000;
            for (let nonce = 0; Date.now() < deadline; nonce++) {
                const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", encoder.encode(`${challenge.token}:${nonce}`)));
                if (digest[0] === 0 && digest[1] === 0) {
                    form.elements.work_token.value = challenge.token;
                    form.elements.work_nonce.value = String(nonce);
                    form.requestSubmit();
                    return;
                }
                if (nonce % 256 === 0) await new Promise(resolve => setTimeout(resolve, 0));
            }
            throw new Error("expired");
        } catch {
            // The server returns a localized, accessible refusal; never bypass proof.
            form.elements.work_token.value = "";
            form.elements.work_nonce.value = "";
            HTMLFormElement.prototype.submit.call(form);
        } finally {
            busy = false;
            form.removeAttribute("aria-busy");
        }
    });
}
