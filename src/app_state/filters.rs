use gpui::*;
use uuid::Uuid;

use super::AppState;
use crate::domain::Filter;

impl AppState {
    pub fn save_filter(
        &mut self,
        id: Option<&str>,
        name: String,
        filename: String,
        content: String,
        cx: &mut Context<Self>,
    ) {
        let duplicate = self.config.filters.iter().any(|item| {
            Some(item.id.as_str()) != id
                && (item.name.eq_ignore_ascii_case(&name)
                    || item.filename.eq_ignore_ascii_case(&filename)
                    || item.content.trim() == content.trim())
        });
        let reserved = self.builtin.filters.iter().any(|item| {
            Some(item.id.as_str()) != id && item.filename.eq_ignore_ascii_case(&filename)
        });
        if name.trim().is_empty() || filename.trim().is_empty() || duplicate || reserved {
            self.set_error(
                anyhow::anyhow!("фильтр пустой, дублируется или использует системное имя файла"),
                cx,
            );
            return;
        }
        let previous = id.and_then(|id| {
            self.config
                .filters
                .iter()
                .find(|item| item.id == id)
                .cloned()
        });
        let filter = if let Some(id) = id {
            let Some(existing) = self.config.filters.iter().find(|item| item.id == id) else {
                return;
            };
            Filter {
                name,
                filename,
                content,
                ..existing.clone()
            }
        } else {
            Filter {
                id: Uuid::new_v4().to_string(),
                name,
                filename,
                content,
                active: false,
                system: false,
                system_base_name: None,
                system_base_filename: None,
                system_base_content: None,
                system_base_active: None,
            }
        };
        match self.repository.save_filter(&filter) {
            Ok(()) => {
                if let Some(previous) = previous
                    && previous.filename != filter.filename
                    && !previous.system
                    && let Err(error) = self.repository.delete_filter(&previous.filename)
                {
                    self.set_error(error, cx);
                    return;
                }
                self.mutate(
                    |config| match config.filters.iter().position(|item| item.id == filter.id) {
                        Some(index) => config.filters[index] = filter,
                        None => config.filters.push(filter),
                    },
                    cx,
                );
                self.apply_connected(cx);
            }
            Err(error) => self.set_error(error, cx),
        }
    }

    pub fn toggle_filter(&mut self, id: &str, cx: &mut Context<Self>) {
        self.mutate(
            |config| {
                if let Some(item) = config.filters.iter_mut().find(|item| item.id == id) {
                    item.active = !item.active;
                }
            },
            cx,
        );
        self.apply_connected(cx);
    }

    pub fn delete_filter(&mut self, id: &str, cx: &mut Context<Self>) {
        let filter = self
            .config
            .filters
            .iter()
            .find(|item| item.id == id)
            .cloned();
        if let Some(filter) = filter
            && !filter.system
            && let Err(error) = self.repository.delete_filter(&filter.filename)
        {
            self.set_error(error, cx);
            return;
        }
        self.mutate(
            |config| {
                if config
                    .filters
                    .iter()
                    .any(|item| item.id == id && item.system)
                {
                    config.system_removed_filter_ids.push(id.to_owned());
                }
                config.filters.retain(|item| item.id != id);
            },
            cx,
        );
        self.apply_connected(cx);
    }

    pub fn restore_filter(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(filter) = self
            .builtin
            .filters
            .iter()
            .find(|item| item.id == id)
            .cloned()
        else {
            return;
        };
        if let Err(error) = self.repository.save_filter(&filter) {
            self.set_error(error, cx);
            return;
        }
        self.mutate(
            |config| {
                match config.filters.iter().position(|item| item.id == id) {
                    Some(index) => config.filters[index] = filter,
                    None => config.filters.push(filter),
                }
                config.system_removed_filter_ids.retain(|item| item != id);
            },
            cx,
        );
        self.apply_connected(cx);
    }
}
