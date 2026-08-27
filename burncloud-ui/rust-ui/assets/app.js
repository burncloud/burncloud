(() => {
  const body = document.body;
  const sidebar = document.getElementById("sidebar");
  const sidebarToggles = document.querySelectorAll('[aria-controls="sidebar"]');
  const sidebarClosers = document.querySelectorAll("[data-close-sidebar]");

  const setSidebarOpen = (open) => {
    body.classList.toggle("sidebar-open", open);
    sidebarToggles.forEach((button) => button.setAttribute("aria-expanded", String(open)));
    if (open) sidebar?.querySelector("a")?.focus();
  };
  sidebarToggles.forEach((button) => button.addEventListener("click", () => setSidebarOpen(true)));
  sidebarClosers.forEach((button) => button.addEventListener("click", () => setSidebarOpen(false)));

  const menuRoots = [...document.querySelectorAll(".menu-root")];
  const closeMenus = (except) => {
    menuRoots.forEach((root) => {
      if (root === except) return;
      root.querySelector("[data-menu-panel]")?.setAttribute("hidden", "");
      root.querySelector("[data-menu-trigger]")?.setAttribute("aria-expanded", "false");
    });
  };
  menuRoots.forEach((root) => {
    const trigger = root.querySelector("[data-menu-trigger]");
    const panel = root.querySelector("[data-menu-panel]");
    if (!trigger || !panel) return;
    trigger.addEventListener("click", (event) => {
      event.stopPropagation();
      const open = panel.hasAttribute("hidden");
      closeMenus(root);
      panel.toggleAttribute("hidden", !open);
      trigger.setAttribute("aria-expanded", String(open));
      if (open) panel.querySelector("a, button")?.focus();
    });
  });
  document.addEventListener("click", (event) => {
    if (!event.target.closest(".menu-root")) closeMenus();
  });

  const globalSearch = document.getElementById("global-search");
  const globalResults = document.getElementById("global-search-results");
  if (globalSearch && globalResults) {
    const existing = new Set([...globalResults.querySelectorAll("a")].map((item) => item.href));
    const models = new Map();
    document.querySelectorAll("[data-model-card]").forEach((card) => {
      const id = card.querySelector("[data-open-model]")?.dataset.openModel;
      if (id) models.set(id, `/buyer/playground?model=${encodeURIComponent(id)}`);
    });
    document.querySelectorAll("#model-select option[value]").forEach((option) => {
      if (option.value) models.set(option.value, `/buyer/playground?model=${encodeURIComponent(option.value)}`);
    });
    models.forEach((href, label) => {
      const absolute = new URL(href, window.location.origin).href;
      if (existing.has(absolute)) return;
      const link = document.createElement("a");
      link.href = href;
      link.dataset.globalResult = "";
      link.dataset.search = `${label} 模型 playground 操练场`;
      link.textContent = `${label} · 模型`;
      globalResults.append(link);
    });

    const updateSearch = () => {
      const query = globalSearch.value.trim().toLocaleLowerCase();
      let visible = 0;
      globalResults.querySelectorAll("[data-global-result]").forEach((item) => {
        const matches = !query || (item.dataset.search || item.textContent).toLocaleLowerCase().includes(query);
        item.hidden = !matches;
        if (matches) visible += 1;
      });
      const open = query.length > 0 && visible > 0;
      globalResults.hidden = !open;
      globalSearch.setAttribute("aria-expanded", String(open));
    };
    globalSearch.addEventListener("input", updateSearch);
    globalSearch.addEventListener("focus", updateSearch);
    globalSearch.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        const first = [...globalResults.querySelectorAll("[data-global-result]")].find((item) => !item.hidden);
        if (first) {
          event.preventDefault();
          first.click();
        }
      } else if (event.key === "Escape") {
        globalResults.hidden = true;
        globalSearch.setAttribute("aria-expanded", "false");
        globalSearch.blur();
      }
    });
    document.addEventListener("click", (event) => {
      if (!event.target.closest(".global-search")) {
        globalResults.hidden = true;
        globalSearch.setAttribute("aria-expanded", "false");
      }
    });
    document.addEventListener("keydown", (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === "k") {
        event.preventDefault();
        globalSearch.focus();
        globalSearch.select();
      }
    });
  }

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      setSidebarOpen(false);
      closeMenus();
    }
  });
})();
