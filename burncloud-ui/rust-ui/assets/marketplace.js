(() => {
  const root = document.querySelector("[data-marketplace]");
  if (!root) return;

  const search = root.querySelector("#model-search");
  const categoryButtons = [...root.querySelectorAll("[data-category]")];
  const cards = [...root.querySelectorAll("[data-model-card]")];
  const resultCount = root.querySelector("#result-count");
  const emptyState = root.querySelector("#marketplace-empty");
  const clearFilters = root.querySelector("#clear-filters");
  const drawerLayer = root.querySelector("#model-drawer");
  const drawerTitle = root.querySelector("#drawer-title");
  const drawerSubtitle = root.querySelector("#drawer-subtitle");
  const drawerBody = root.querySelector("#drawer-body");
  let activeCategory = "all";
  let lastFocused = null;

  const normalize = (value) => value.trim().toLocaleLowerCase();

  const updateResults = () => {
    const query = normalize(search.value);
    let visibleCount = 0;

    cards.forEach((card) => {
      const matchesCategory = activeCategory === "all" || card.dataset.category === activeCategory;
      const matchesSearch = !query || normalize(card.dataset.search).includes(query);
      const visible = matchesCategory && matchesSearch;
      card.hidden = !visible;
      if (visible) visibleCount += 1;
    });

    resultCount.textContent = visibleCount ? `找到 ${visibleCount} 个可用模型` : "未找到匹配模型";
    emptyState.hidden = visibleCount !== 0;
  };

  const selectCategory = (selected) => {
    activeCategory = selected.dataset.category;
    categoryButtons.forEach((button) => {
      const active = button === selected;
      button.classList.toggle("active", active);
      button.setAttribute("aria-pressed", String(active));
    });
    updateResults();
  };

  const syncModelQuery = (modelId) => {
    const url = new URL(window.location.href);
    if (modelId) url.searchParams.set("model", modelId);
    else url.searchParams.delete("model");
    window.history.replaceState({}, "", url);
  };

  const closeDrawer = (syncQuery = true) => {
    if (drawerLayer.hidden) return;
    drawerLayer.hidden = true;
    document.body.classList.remove("drawer-open");
    drawerBody.replaceChildren();
    if (syncQuery) syncModelQuery(null);
    lastFocused?.focus();
  };

  const bindAdvancedToggle = () => {
    const toggle = drawerBody.querySelector(".advanced-toggle");
    const details = drawerBody.querySelector(".advanced-section dl");
    if (!toggle || !details) return;
    toggle.addEventListener("click", () => {
      const expanded = toggle.getAttribute("aria-expanded") === "true";
      toggle.setAttribute("aria-expanded", String(!expanded));
      details.hidden = expanded;
    });
  };

  const openDrawer = (button, syncQuery = true) => {
    const template = root.querySelector(`#model-detail-${CSS.escape(button.dataset.openModel)}`);
    if (!template) return;
    lastFocused = button;
    drawerTitle.textContent = template.dataset.title;
    drawerSubtitle.textContent = template.dataset.subtitle;
    drawerBody.replaceChildren(template.content.cloneNode(true));
    drawerLayer.hidden = false;
    document.body.classList.add("drawer-open");
    if (syncQuery) syncModelQuery(button.dataset.openModel);
    bindAdvancedToggle();
    drawerLayer.querySelector("[data-close-drawer]").focus();
  };

  categoryButtons.forEach((button) => button.addEventListener("click", () => selectCategory(button)));
  search.addEventListener("input", updateResults);
  root.querySelectorAll("[data-open-model]").forEach((button) => {
    button.addEventListener("click", () => openDrawer(button));
  });
  root.querySelectorAll("[data-close-drawer]").forEach((button) => button.addEventListener("click", closeDrawer));

  clearFilters.addEventListener("click", () => {
    search.value = "";
    selectCategory(categoryButtons[0]);
    search.focus();
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !drawerLayer.hidden) {
      closeDrawer();
      return;
    }
    if (event.key === "/" && drawerLayer.hidden && !["INPUT", "TEXTAREA", "SELECT"].includes(document.activeElement.tagName)) {
      event.preventDefault();
      search.focus();
    }
  });

  updateResults();
  const linkedModel = new URLSearchParams(window.location.search).get("model");
  if (linkedModel) {
    const linkedButton = root.querySelector(`[data-open-model="${CSS.escape(linkedModel)}"]`);
    if (linkedButton) openDrawer(linkedButton, false);
    else syncModelQuery(null);
  }
})();
