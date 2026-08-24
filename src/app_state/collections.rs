use gpui::*;
use uuid::Uuid;

use super::AppState;
use crate::domain::{Category, Placeholder, Strategy};

impl AppState {
    pub fn save_category(&mut self, id: Option<&str>, name: String, cx: &mut Context<Self>) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        if name.trim().is_empty()
            || self
                .config
                .categories
                .iter()
                .any(|item| Some(item.id.as_str()) != id && item.name.eq_ignore_ascii_case(&name))
        {
            self.set_error(
                anyhow::anyhow!("название категории пустое или уже используется"),
                cx,
            );
            return;
        }
        self.mutate(
            |config| match id {
                Some(id) => {
                    if let Some(item) = config.categories.iter_mut().find(|item| item.id == id) {
                        item.name = name;
                    }
                }
                None => config.categories.push(Category {
                    id: Uuid::new_v4().to_string(),
                    name,
                    strategies: Vec::new(),
                    system: false,
                    system_base_name: None,
                }),
            },
            cx,
        );
    }

    pub fn delete_category(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        let mut removed_strategies = Vec::new();
        self.mutate(
            |config| {
                if let Some(category) = config.categories.iter().find(|item| item.id == id) {
                    if category.system {
                        config.system_removed_category_ids.push(id.to_owned());
                    }
                    removed_strategies = category.strategies.clone();
                }
                config.categories.retain(|item| item.id != id);
            },
            cx,
        );
        for strat in removed_strategies {
            if let Err(error) = self.repository.delete_strategy(&strat.id) {
                self.set_error(error, cx);
                return;
            }
        }
        self.apply_connected(cx);
    }

    pub fn reorder_category(&mut self, id: &str, target: usize, cx: &mut Context<Self>) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        self.mutate(
            |config| {
                let Some(index) = config.categories.iter().position(|item| item.id == id) else {
                    return;
                };
                let category = config.categories.remove(index);
                config
                    .categories
                    .insert(target.min(config.categories.len()), category);
            },
            cx,
        );
    }

    pub fn clear_category(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        let mut affected = Vec::new();
        self.mutate(
            |config| {
                if let Some(category) = config.categories.iter_mut().find(|item| item.id == id) {
                    for strategy in &mut category.strategies {
                        if strategy.active {
                            strategy.active = false;
                            affected.push(strategy.clone());
                        }
                    }
                }
            },
            cx,
        );
        for strat in affected {
            if let Err(error) = self.repository.save_strategy(&strat) {
                self.set_error(error, cx);
                return;
            }
        }
        self.apply_connected(cx);
    }

    pub fn restore_category(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        let Some(builtin) = self
            .builtin
            .categories
            .iter()
            .find(|item| item.id == id)
            .cloned()
        else {
            return;
        };
        for strat in &builtin.strategies {
            if let Err(error) = self.repository.save_strategy(strat) {
                self.set_error(error, cx);
                return;
            }
        }
        self.mutate(
            |config| {
                match config.categories.iter().position(|item| item.id == id) {
                    Some(index) => config.categories[index] = builtin,
                    None => config.categories.push(builtin),
                }
                config.system_removed_category_ids.retain(|item| item != id);
                config
                    .system_removed_strategy_keys
                    .retain(|key| !key.starts_with(&format!("{id}::")));
            },
            cx,
        );
        self.apply_connected(cx);
    }

    pub fn save_strategy(
        &mut self,
        category_id: &str,
        id: Option<&str>,
        name: String,
        content: String,
        cx: &mut Context<Self>,
    ) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        let duplicate = self
            .config
            .categories
            .iter()
            .find(|item| item.id == category_id)
            .is_some_and(|category| {
                category.strategies.iter().any(|item| {
                    Some(item.id.as_str()) != id
                        && (item.name.eq_ignore_ascii_case(&name)
                            || item.content.trim() == content.trim())
                })
            });
        if name.trim().is_empty() || content.trim().is_empty() || duplicate {
            self.set_error(
                anyhow::anyhow!("стратегия пустая либо дублирует существующую"),
                cx,
            );
            return;
        }
        let mut target_strategy: Option<Strategy> = None;
        self.mutate(
            |config| {
                if let Some(category) = config
                    .categories
                    .iter_mut()
                    .find(|item| item.id == category_id)
                {
                    match id {
                        Some(id) => {
                            if let Some(item) =
                                category.strategies.iter_mut().find(|item| item.id == id)
                            {
                                item.name = name;
                                item.content = content;
                                target_strategy = Some(item.clone());
                            }
                        }
                        None => {
                            let item = Strategy {
                                id: Uuid::new_v4().to_string(),
                                name,
                                category: category.name.clone(),
                                category_id: category.id.clone(),
                                category_order: None,
                                order: None,
                                description: None,
                                content,
                                active: false,
                                system: false,
                                system_base_name: None,
                                system_base_content: None,
                            };
                            target_strategy = Some(item.clone());
                            category.strategies.push(item);
                        }
                    }
                }
            },
            cx,
        );
        if let Some(strat) = target_strategy
            && let Err(error) = self.repository.save_strategy(&strat)
        {
            self.set_error(error, cx);
            return;
        }
        self.apply_connected(cx);
    }

    pub fn select_strategy(
        &mut self,
        category_id: &str,
        strategy_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        let mut affected: Vec<Strategy> = Vec::new();
        self.mutate(
            |config| {
                if let Some(category) = config
                    .categories
                    .iter_mut()
                    .find(|item| item.id == category_id)
                {
                    let disable = category
                        .strategies
                        .iter()
                        .any(|item| item.id == strategy_id && item.active);
                    for item in &mut category.strategies {
                        let new_active = !disable && item.id == strategy_id;
                        if item.active != new_active {
                            item.active = new_active;
                            affected.push(item.clone());
                        }
                    }
                }
            },
            cx,
        );
        for strat in affected {
            if let Err(error) = self.repository.save_strategy(&strat) {
                self.set_error(error, cx);
                return;
            }
        }
        self.apply_connected(cx);
    }

    pub fn delete_strategy(
        &mut self,
        category_id: &str,
        strategy_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        self.mutate(
            |config| {
                if let Some(category) = config
                    .categories
                    .iter_mut()
                    .find(|item| item.id == category_id)
                {
                    if category
                        .strategies
                        .iter()
                        .any(|item| item.id == strategy_id && item.system)
                    {
                        config
                            .system_removed_strategy_keys
                            .push(format!("{category_id}::{strategy_id}"));
                    }
                    category.strategies.retain(|item| item.id != strategy_id);
                }
            },
            cx,
        );
        if let Err(error) = self.repository.delete_strategy(strategy_id) {
            self.set_error(error, cx);
            return;
        }
        self.apply_connected(cx);
    }

    pub fn restore_strategy(
        &mut self,
        category_id: &str,
        strategy_id: &str,
        cx: &mut Context<Self>,
    ) {
        if self.strategy_edits_blocked(cx) {
            return;
        }
        let Some(strategy) = self
            .builtin
            .categories
            .iter()
            .find(|item| item.id == category_id)
            .and_then(|category| {
                category
                    .strategies
                    .iter()
                    .find(|item| item.id == strategy_id)
            })
            .cloned()
        else {
            return;
        };
        if let Err(error) = self.repository.save_strategy(&strategy) {
            self.set_error(error, cx);
            return;
        }
        self.mutate(
            |config| {
                if let Some(category) = config
                    .categories
                    .iter_mut()
                    .find(|item| item.id == category_id)
                {
                    match category
                        .strategies
                        .iter()
                        .position(|item| item.id == strategy_id)
                    {
                        Some(index) => category.strategies[index] = strategy,
                        None => category.strategies.push(strategy),
                    }
                }
                config
                    .system_removed_strategy_keys
                    .retain(|key| key != &format!("{category_id}::{strategy_id}"));
            },
            cx,
        );
        self.apply_connected(cx);
    }

    pub fn save_placeholder(
        &mut self,
        index: Option<usize>,
        name: String,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let duplicate = self
            .config
            .placeholders
            .iter()
            .enumerate()
            .any(|(item_index, item)| {
                Some(item_index) != index
                    && (item.name.eq_ignore_ascii_case(&name)
                        || item.path.eq_ignore_ascii_case(&path))
            });
        if name.trim().is_empty() || path.trim().is_empty() || duplicate {
            self.set_error(
                anyhow::anyhow!("плейсхолдер пустой либо уже существует"),
                cx,
            );
            return;
        }
        self.mutate(
            |config| match index {
                Some(index) if index < config.placeholders.len() => {
                    config.placeholders[index].name = name;
                    config.placeholders[index].path = path;
                }
                _ => config.placeholders.push(Placeholder {
                    name,
                    path,
                    system: false,
                    system_base_name: None,
                    system_base_path: None,
                }),
            },
            cx,
        );
        self.apply_connected(cx);
    }

    pub fn delete_placeholder(&mut self, index: usize, cx: &mut Context<Self>) {
        self.mutate(
            |config| {
                if index < config.placeholders.len() {
                    if config.placeholders[index].system {
                        config
                            .system_removed_placeholder_names
                            .push(config.placeholders[index].name.clone());
                    }
                    config.placeholders.remove(index);
                }
            },
            cx,
        );
        self.apply_connected(cx);
    }

    pub fn restore_placeholder(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(placeholder) = self
            .builtin
            .placeholders
            .iter()
            .find(|item| item.name == name)
            .cloned()
        else {
            return;
        };
        self.mutate(
            |config| {
                match config
                    .placeholders
                    .iter()
                    .position(|item| item.name == name)
                {
                    Some(index) => config.placeholders[index] = placeholder,
                    None => config.placeholders.push(placeholder),
                }
                config
                    .system_removed_placeholder_names
                    .retain(|item| item != name);
            },
            cx,
        );
        self.apply_connected(cx);
    }
}
