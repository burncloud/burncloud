//! One-page-at-a-time progression for jobs-aesthetic loop.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::gates::GateCategory;

pub const PAGE_ORDER: &[&str] = GateCategory::JOBS_AESTHETIC_PAGES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageProgress {
    pub completed_pages: Vec<String>,
    pub current_page: String,
    /// When set, loop only works through this subset (e.g. home-only pilot).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_pages: Option<Vec<String>>,
    pub updated_at: String,
}

impl PageProgress {
    pub fn new() -> Self {
        Self::with_scope(None)
    }

    pub fn with_scope(only_pages: Option<Vec<String>>) -> Self {
        let order = effective_order(only_pages.as_deref());
        let first = order
            .first()
            .copied()
            .unwrap_or(PAGE_ORDER[0])
            .to_string();
        Self {
            completed_pages: Vec::new(),
            current_page: first,
            active_pages: only_pages,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(Self::new)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut copy = self.clone();
        copy.updated_at = Utc::now().to_rfc3339();
        std::fs::write(path, serde_json::to_string_pretty(&copy)?)?;
        Ok(())
    }

    pub fn progress_path(run_dir: &Path) -> PathBuf {
        run_dir.join("page-progress.json")
    }

    pub fn page_order(&self) -> Vec<&'static str> {
        if let Some(ref pages) = self.active_pages {
            pages
                .iter()
                .filter_map(|p| PAGE_ORDER.iter().copied().find(|k| k == p))
                .collect()
        } else {
            PAGE_ORDER.to_vec()
        }
    }

    pub fn all_complete(&self) -> bool {
        let order = self.page_order();
        order
            .iter()
            .all(|p| self.completed_pages.iter().any(|c| c == p))
    }

    pub fn is_last_page(&self) -> bool {
        let order = self.page_order();
        order.last().is_some_and(|last| self.current_page == *last)
    }

    /// Mark the current page done and advance to the next incomplete page in scope.
    pub fn complete_current(&mut self) -> Option<String> {
        let done = self.current_page.clone();
        if !self.completed_pages.iter().any(|p| p == &done) {
            self.completed_pages.push(done.clone());
        }
        let order = self.page_order();
        self.current_page = order
            .iter()
            .find(|p| !self.completed_pages.iter().any(|c| c == *p))
            .map(|s| (*s).to_string())
            .unwrap_or_else(|| done.clone());
        Some(done)
    }

    pub fn remaining_count(&self) -> usize {
        let order = self.page_order();
        order
            .iter()
            .filter(|p| !self.completed_pages.iter().any(|c| c == *p))
            .count()
    }

    /// Apply a page scope and reset queue to the first incomplete page in that scope.
    pub fn apply_scope(&mut self, only_pages: Vec<String>) -> anyhow::Result<()> {
        for p in &only_pages {
            if !PAGE_ORDER.contains(&p.as_str()) {
                anyhow::bail!("unknown page key '{p}'. Valid: {}", PAGE_ORDER.join(", "));
            }
        }
        self.active_pages = Some(only_pages);
        let order = self.page_order();
        self.completed_pages
            .retain(|p| order.iter().any(|k| *k == p));
        if !order.iter().any(|p| *p == self.current_page.as_str()) {
            self.current_page = order
                .first()
                .copied()
                .unwrap_or(PAGE_ORDER[0])
                .to_string();
        }
        Ok(())
    }
}

fn effective_order(only_pages: Option<&[String]>) -> Vec<&'static str> {
    match only_pages {
        Some(pages) if !pages.is_empty() => pages
            .iter()
            .filter_map(|p| PAGE_ORDER.iter().copied().find(|k| k == p))
            .collect(),
        _ => PAGE_ORDER.to_vec(),
    }
}

impl Default for PageProgress {
    fn default() -> Self {
        Self::new()
    }
}
