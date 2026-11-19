"use strict";

function enablePortalNavigation(header) {
  const nav = header.querySelector("#portal-navigation");
  const toggle = header.querySelector("#portal-menu-toggle");
  const tools = nav.querySelector(".om-tools");
  const download = tools.querySelector(".om-download");
  const mobile = window.matchMedia("(max-width: 960px)");
  const closeLabel = toggle.dataset.closeLabel || (document.documentElement.lang === "pl" ? "Zamknij" : "Close");
  const toggleLabel = toggle.querySelector("span");
  const menuLabel = toggleLabel.textContent;
  const toggleIcon = toggle.querySelector("use");
  const menuIcon = toggleIcon.getAttribute("href");
  const closeIcon = menuIcon.replace(/#ic-menu$/, "#ic-close");
  const account = document.getElementById("podglad-konta");
  let active = null;
  let accountTrigger = null;

  function panelFor(button) {
    return document.getElementById(button.getAttribute("aria-controls"));
  }

  function showGroup(button) {
    if (active) {
      active.setAttribute("aria-expanded", "false");
      panelFor(active).hidden = true;
    }
    active = button;
    if (active) {
      active.setAttribute("aria-expanded", "true");
      panelFor(active).hidden = false;
    }
  }

  function showMobile(open) {
    if (!open) showGroup(null);
    header.classList.toggle("is-open", open);
    toggle.setAttribute("aria-expanded", String(open));
    toggleLabel.textContent = open ? closeLabel : menuLabel;
    toggleIcon.setAttribute("href", open ? closeIcon : menuIcon);
  }

  function arrangeNav() {
    const previous = document.activeElement;
    showMobile(false);
    // Keep keyboard order consistent with the visible order when tools move.
    if (mobile.matches) {
      nav.prepend(tools);
      if (download) tools.append(download);
      if (nav.contains(previous)) toggle.focus();
    } else {
      nav.append(tools);
      if (download) tools.prepend(download);
      if (previous === toggle || previous.closest(".om-panel")) {
        const group = previous.closest(".om-section");
        (group?.querySelector("[data-nav-section]") || header.querySelector(".om-brand")).focus();
      }
    }
  }

  toggle.addEventListener("click", () => showMobile(!header.classList.contains("is-open")));
  for (const button of nav.querySelectorAll("[data-nav-section]")) {
    const panel = panelFor(button);
    button.addEventListener("click", () => showGroup(active === button ? null : button));
    button.addEventListener("keydown", event => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      const links = panel.querySelectorAll("a[href]");
      if (!links.length) return;
      event.preventDefault();
      showGroup(button);
      links[event.key === "ArrowUp" ? links.length - 1 : 0].focus();
    });
  }

  header.addEventListener("keydown", event => {
    if (event.key === "Escape") {
      if (active) {
        const button = active;
        showGroup(null);
        button.focus();
      } else if (header.classList.contains("is-open")) {
        showMobile(false);
        toggle.focus();
      } else return;
      event.preventDefault();
      return;
    }
    if (!active || !panelFor(active).contains(event.target)) return;
    const links = [...panelFor(active).querySelectorAll("a[href]")];
    const index = links.indexOf(event.target);
    if (index < 0) return;
    const next = {
      ArrowDown: (index + 1) % links.length,
      ArrowUp: (index - 1 + links.length) % links.length,
      Home: 0,
      End: links.length - 1
    }[event.key];
    if (typeof next === "number") {
      event.preventDefault();
      links[next].focus();
    }
  });

  document.addEventListener("focusin", event => {
    if (active && !active.closest(".om-section").contains(event.target)) showGroup(null);
    if (header.classList.contains("is-open") && !header.contains(event.target)) showMobile(false);
  });

  document.addEventListener("click", event => {
    if (!header.contains(event.target)) {
      if (active || header.classList.contains("is-open")) showMobile(false);
      return;
    }
    const link = event.target.closest("a[href]");
    if (!link || event.defaultPrevented || event.button !== 0 ||
        event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    showMobile(false);
    if (account && typeof account.showModal === "function" && link.hasAttribute("data-account-link")) {
      event.preventDefault();
      accountTrigger = link;
      account.showModal();
      return;
    }
    const href = link.getAttribute("href");
    if (href.startsWith("#") && href.length > 1) {
      const destination = document.getElementById(href.slice(1));
      if (destination) {
        if (!destination.hasAttribute("tabindex")) destination.tabIndex = -1;
        destination.focus({preventScroll: true});
      }
    }
  });

  if (account) {
    for (const close of account.querySelectorAll(".account-close")) {
      close.addEventListener("click", () => account.close());
    }
    account.addEventListener("click", event => {
      if (event.target === account) account.close();
    });
    account.addEventListener("close", () => {
      const target = accountTrigger?.getClientRects().length ? accountTrigger : toggle;
      target.focus({preventScroll: true});
    });
  }

  header.classList.add("is-enhanced");
  mobile.addEventListener("change", arrangeNav);
  arrangeNav();
}

function enableProfessionTabs(tabList) {
  const tabs = [...tabList.querySelectorAll('[role="tab"]')];
  const panel = document.getElementById(tabs[0].getAttribute("aria-controls"));
  const outfit = panel.querySelector(".outfit");
  const name = panel.querySelector("h3");
  const description = panel.querySelector("p");

  function select(tab) {
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
    }
    panel.setAttribute("aria-labelledby", tab.id);
    outfit.src = tab.dataset.outfit;
    name.textContent = tab.querySelector(".pname").textContent;
    description.textContent = tab.dataset.description;
  }

  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => select(tab));
    tab.addEventListener("keydown", event => {
      const next = {
        ArrowRight: (index + 1) % tabs.length,
        ArrowLeft: (index - 1 + tabs.length) % tabs.length,
        Home: 0,
        End: tabs.length - 1
      }[event.key];
      if (typeof next === "number") {
        event.preventDefault();
        select(tabs[next]);
        tabs[next].focus();
      }
    });
  });
}

const homeHeader = document.getElementById("portal-header");
if (homeHeader) enablePortalNavigation(homeHeader);
const professionTabs = document.querySelector('.portraits[role="tablist"]');
if (professionTabs?.querySelector('[role="tab"]')) enableProfessionTabs(professionTabs);
