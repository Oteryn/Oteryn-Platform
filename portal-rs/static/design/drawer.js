/* Native disclosure fallback; dialog supplies focus trapping, Escape and inertness. */
(() => {
  "use strict";
  if (typeof HTMLDialogElement === "undefined" || !HTMLDialogElement.prototype.showModal) return;
  for (const details of document.querySelectorAll("details[data-overlay]")) {
    if (details.dataset.enhanced) continue;
    const summary = details.querySelector("summary");
    const panel = details.querySelector("section");
    const close = panel?.querySelector("[data-close]");
    if (!summary || !panel || !close) continue;
    const dialog = document.createElement("dialog");
    dialog.className = "op-dialog" + (details.dataset.overlay === "drawer" ? " op-dialog--drawer" : "");
    dialog.id = details.id + "-dialog";
    dialog.setAttribute("aria-labelledby", panel.getAttribute("aria-labelledby"));
    const content = document.createElement("div");
    content.className = "op-stack";
    content.append(...panel.childNodes);
    dialog.append(content);
    const trigger = document.createElement("button");
    trigger.type = "button";
    trigger.className = "op-button op-button--secondary";
    trigger.textContent = summary.textContent;
    trigger.setAttribute("aria-haspopup", "dialog");
    trigger.setAttribute("aria-controls", dialog.id);
    trigger.setAttribute("aria-expanded", "false");
    trigger.addEventListener("click", () => {
      dialog.showModal();
      trigger.setAttribute("aria-expanded", "true");
    });
    close.hidden = false;
    close.addEventListener("click", () => dialog.close());
    dialog.addEventListener("close", () => {
      trigger.setAttribute("aria-expanded", "false");
      trigger.focus();
    });
    details.before(trigger);
    details.after(dialog);
    details.hidden = true;
    details.dataset.enhanced = "true";
  }
})();
