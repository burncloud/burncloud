use super::model::{MarketplaceModel, ModelCategory, MODEL_CATALOG};

#[derive(Clone, Debug, PartialEq)]
pub struct MarketplaceState {
    pub search: String,
    pub category: ModelCategory,
    pub selected_model_id: Option<&'static str>,
    pub drawer_open: bool,
    pub slo_expanded: bool,
}

impl Default for MarketplaceState {
    fn default() -> Self {
        Self {
            search: String::new(),
            category: ModelCategory::All,
            selected_model_id: None,
            drawer_open: false,
            slo_expanded: false,
        }
    }
}

impl MarketplaceState {
    pub fn set_search(&mut self, search: impl Into<String>) {
        self.search = search.into();
    }

    pub fn set_category(&mut self, category: ModelCategory) {
        self.category = category;
    }

    pub fn filtered_models(&self) -> Vec<&'static MarketplaceModel> {
        let query = self.search.trim().to_ascii_lowercase();
        MODEL_CATALOG
            .iter()
            .filter(|model| {
                let matches_query = query.is_empty()
                    || model.name.to_ascii_lowercase().contains(&query)
                    || model.family.to_ascii_lowercase().contains(&query)
                    || model.tagline.to_ascii_lowercase().contains(&query);
                let matches_category = match self.category.catalog_name() {
                    Some(category) => model.category.catalog_name() == Some(category),
                    None => true,
                };
                matches_query && matches_category
            })
            .collect()
    }

    pub fn selected_model(&self) -> Option<MarketplaceModel> {
        self.selected_model_id
            .and_then(|id| MODEL_CATALOG.iter().find(|model| model.id == id).copied())
    }

    pub fn open_details(&mut self, model_id: &'static str) {
        self.selected_model_id = Some(model_id);
        self.drawer_open = true;
        self.slo_expanded = false;
    }

    pub fn close_drawer(&mut self) {
        self.drawer_open = false;
        self.slo_expanded = false;
    }

    pub fn toggle_slo(&mut self) {
        self.slo_expanded = !self.slo_expanded;
    }
}
