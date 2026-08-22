use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::*;

use crate::app_state::AppState;
use crate::domain::Category;
use crate::ui::foundation::colors::yellow as accent;

pub(super) use super::category_drag_visual::render_preview;
use crate::ui::components::dashed_outline::dashed_outline;

#[derive(Clone)]
pub(super) struct CategoryDrag {
    pub id: String,
    pub from_index: usize,
    pub category: Category,
    pub source_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub list_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    pub grab_offset: Rc<Cell<Point<Pixels>>>,
}

#[derive(Clone, Copy)]
pub(super) struct ActiveCategoryDrag {
    pub from_index: usize,
    pub placeholder_index: usize,
}

#[derive(Default)]
struct CategoryDragUiState {
    active: Option<ActiveCategoryDrag>,
    dragged: Option<CategoryDrag>,
    mouse_position: Point<Pixels>,
    list_bounds: Option<Bounds<Pixels>>,
    transition: Option<CategoryLayoutTransition>,
    transition_revision: u64,
    row_stride: Pixels,
}

impl Global for CategoryDragUiState {}

pub(super) enum ProjectedCategory {
    Item { index: usize, category: Category },
    Placeholder(Category),
}

#[derive(Clone, Copy)]
pub(super) enum ProjectedCategoryId {
    Item(usize),
    Placeholder,
}

#[derive(Clone, Copy)]
pub(super) struct CategoryLayoutTransition {
    from_index: usize,
    previous_placeholder_index: usize,
    placeholder_index: usize,
    revision: u64,
    row_stride: Pixels,
}

pub(super) struct DragPreviewLayout {
    pub(super) category: Category,
    pub(super) top: Pixels,
    pub(super) height: Pixels,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DropEdge {
    Before,
    After,
}

pub(super) struct InvisibleDragPreview;

impl Render for InvisibleDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size(px(1.)).opacity(0.)
    }
}

pub(super) fn init(cx: &mut App) {
    cx.set_global(CategoryDragUiState::default());
}

pub(super) fn active(cx: &App) -> Option<ActiveCategoryDrag> {
    cx.has_active_drag()
        .then(|| cx.global::<CategoryDragUiState>().active)
        .flatten()
}

pub(super) fn begin(dragged: &CategoryDrag, mouse_position: Point<Pixels>, cx: &mut App) {
    let state = cx.global_mut::<CategoryDragUiState>();
    state.active = Some(ActiveCategoryDrag {
        from_index: dragged.from_index,
        placeholder_index: dragged.from_index,
    });
    state.dragged = Some(dragged.clone());
    state.mouse_position = mouse_position;
    state.transition = None;
    state.row_stride = dragged
        .source_bounds
        .get()
        .map_or(px(84.), |bounds| bounds.size.height + px(12.));
}

pub(super) fn update_mouse_position(position: Point<Pixels>, cx: &mut App) -> bool {
    if !cx.has_active_drag() {
        return false;
    }
    let state = cx.global_mut::<CategoryDragUiState>();
    if state.active.is_none() || state.mouse_position == position {
        return false;
    }
    state.mouse_position = position;
    true
}

pub(super) fn set_list_bounds(bounds: Bounds<Pixels>, cx: &mut App) {
    cx.global_mut::<CategoryDragUiState>().list_bounds = Some(bounds);
}

pub(super) fn preview_layout(cx: &App) -> Option<DragPreviewLayout> {
    if !cx.has_active_drag() {
        return None;
    }
    let state = cx.global::<CategoryDragUiState>();
    let dragged = state.dragged.as_ref()?;
    let source_bounds = dragged.source_bounds.get().unwrap_or(Bounds {
        origin: Point::default(),
        size: size(px(600.), px(72.)),
    });
    let list_bounds = state
        .list_bounds
        .or_else(|| dragged.list_bounds.get())
        .unwrap_or(Bounds {
            origin: Point::default(),
            size: size(px(800.), px(600.)),
        });
    let grab_y = dragged.grab_offset.get().y;
    let card_y = (state.mouse_position.y - grab_y).clamp(
        list_bounds.top(),
        (list_bounds.bottom() - source_bounds.size.height).max(list_bounds.top()),
    );

    Some(DragPreviewLayout {
        category: dragged.category.clone(),
        top: card_y - list_bounds.top(),
        height: source_bounds.size.height,
    })
}

pub(super) fn layout_transition(cx: &App) -> Option<CategoryLayoutTransition> {
    cx.has_active_drag()
        .then(|| cx.global::<CategoryDragUiState>().transition)
        .flatten()
}

impl ProjectedCategory {
    pub(super) fn id(&self) -> ProjectedCategoryId {
        match self {
            Self::Item { index, .. } => ProjectedCategoryId::Item(*index),
            Self::Placeholder(_) => ProjectedCategoryId::Placeholder,
        }
    }
}

pub(super) fn projected_categories(
    categories: &[Category],
    drag: Option<ActiveCategoryDrag>,
) -> Vec<ProjectedCategory> {
    let Some(drag) = drag.filter(|drag| drag.from_index < categories.len()) else {
        return categories
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, category)| ProjectedCategory::Item { index, category })
            .collect();
    };

    let dragged = categories[drag.from_index].clone();
    projected_indices(categories.len(), drag)
        .into_iter()
        .map(|index| match index {
            Some(index) => ProjectedCategory::Item {
                index,
                category: categories[index].clone(),
            },
            None => ProjectedCategory::Placeholder(dragged.clone()),
        })
        .collect()
}

pub(super) fn animate_row(
    row: AnyElement,
    row_id: ProjectedCategoryId,
    transition: Option<CategoryLayoutTransition>,
) -> AnyElement {
    let Some(transition) = transition else {
        return row;
    };
    let initial_offset = row_initial_offset(transition, row_id);
    if initial_offset == px(0.) {
        return row;
    }

    let row_name = match row_id {
        ProjectedCategoryId::Item(index) => format!("item-{index}"),
        ProjectedCategoryId::Placeholder => "placeholder".to_owned(),
    };
    div()
        .relative()
        .w_full()
        .with_animation(
            ElementId::NamedInteger(
                SharedString::from(format!("category-layout-{row_name}")),
                transition.revision,
            ),
            Animation::new(Duration::from_millis(160)).with_easing(ease_in_out),
            move |element, delta| element.top(initial_offset * (1. - delta)),
        )
        .child(row)
        .into_any_element()
}

pub(super) fn placeholder(category: Category, state: Entity<AppState>) -> AnyElement {
    div()
        .id(SharedString::from(format!(
            "category-placeholder-{}",
            category.id
        )))
        .relative()
        .w_full()
        .h(px(72.))
        .rounded(px(8.))
        .bg(accent().opacity(0.025))
        .child(dashed_outline(accent().opacity(0.7).into()))
        .can_drop(|value, _, cx| {
            let Some(dragged) = value.downcast_ref::<CategoryDrag>() else {
                return false;
            };
            cx.global::<CategoryDragUiState>()
                .active
                .is_some_and(|active| active.placeholder_index != dragged.from_index)
        })
        .on_drop(move |dragged: &CategoryDrag, _, cx| {
            let target = cx
                .global::<CategoryDragUiState>()
                .active
                .map(|active| active.placeholder_index);
            if let Some(target) = target.filter(|target| *target != dragged.from_index) {
                state.update(cx, |state, cx| {
                    state.reorder_category(&dragged.id, target, cx)
                });
            }
            finish(cx);
        })
        .into_any_element()
}

pub(super) fn drop_zones(index: usize, state: Entity<AppState>) -> [AnyElement; 2] {
    [
        drop_zone(index, DropEdge::Before, state.clone()).into_any_element(),
        drop_zone(index, DropEdge::After, state).into_any_element(),
    ]
}

fn drop_zone(index: usize, edge: DropEdge, state: Entity<AppState>) -> Stateful<Div> {
    let edge_name = match edge {
        DropEdge::Before => "before",
        DropEdge::After => "after",
    };
    div()
        .id(SharedString::from(format!(
            "category-drop-{edge_name}-{index}"
        )))
        .absolute()
        .left_0()
        .right_0()
        .h(relative(0.5))
        .when(edge == DropEdge::Before, |zone| zone.top_0())
        .when(edge == DropEdge::After, |zone| zone.bottom_0())
        .on_drag_move::<CategoryDrag>(move |event, window, cx| {
            let from_index = event.drag(cx).from_index;
            update_mouse_position(event.event.position, cx);
            if event.bounds.contains(&event.event.position) {
                let placeholder_index = drop_index(from_index, index, edge);
                move_placeholder(from_index, placeholder_index, cx);
            }
            window.refresh();
        })
        .can_drop(move |value, _, _| {
            value
                .downcast_ref::<CategoryDrag>()
                .and_then(|dragged| reorder_index(dragged.from_index, index, edge))
                .is_some()
        })
        .on_drop(move |dragged: &CategoryDrag, _, cx| {
            let Some(target) = reorder_index(dragged.from_index, index, edge) else {
                return;
            };
            state.update(cx, |state, cx| {
                state.reorder_category(&dragged.id, target, cx)
            });
            finish(cx);
        })
}

fn finish(cx: &mut App) {
    let state = cx.global_mut::<CategoryDragUiState>();
    state.active = None;
    state.dragged = None;
    state.transition = None;
    cx.refresh_windows();
}

fn move_placeholder(from_index: usize, placeholder_index: usize, cx: &mut App) -> bool {
    let state = cx.global_mut::<CategoryDragUiState>();
    let Some(active) = state.active else {
        return false;
    };
    if active.from_index != from_index || active.placeholder_index == placeholder_index {
        return false;
    }
    state.transition_revision = state.transition_revision.wrapping_add(1);
    state.transition = Some(CategoryLayoutTransition {
        from_index,
        previous_placeholder_index: active.placeholder_index,
        placeholder_index,
        revision: state.transition_revision,
        row_stride: state.row_stride,
    });
    state.active = Some(ActiveCategoryDrag {
        from_index,
        placeholder_index,
    });
    true
}

fn row_initial_offset(transition: CategoryLayoutTransition, row_id: ProjectedCategoryId) -> Pixels {
    let previous = projected_indices(
        transition_length(transition),
        ActiveCategoryDrag {
            from_index: transition.from_index,
            placeholder_index: transition.previous_placeholder_index,
        },
    );
    let current = projected_indices(
        transition_length(transition),
        ActiveCategoryDrag {
            from_index: transition.from_index,
            placeholder_index: transition.placeholder_index,
        },
    );
    let needle = match row_id {
        ProjectedCategoryId::Item(index) => Some(index),
        ProjectedCategoryId::Placeholder => None,
    };
    let previous_position = previous.iter().position(|entry| *entry == needle);
    let current_position = current.iter().position(|entry| *entry == needle);
    match (previous_position, current_position) {
        (Some(previous), Some(current)) if previous > current => {
            (current..previous).fold(px(0.), |offset, _| offset + transition.row_stride)
        }
        (Some(previous), Some(current)) if current > previous => {
            (previous..current).fold(px(0.), |offset, _| offset - transition.row_stride)
        }
        _ => px(0.),
    }
}

fn transition_length(transition: CategoryLayoutTransition) -> usize {
    transition
        .from_index
        .max(transition.previous_placeholder_index)
        .max(transition.placeholder_index)
        + 1
}

fn projected_indices(len: usize, drag: ActiveCategoryDrag) -> Vec<Option<usize>> {
    if len == 0 || drag.from_index >= len {
        return Vec::new();
    }
    let mut indices = (0..len)
        .filter(|index| *index != drag.from_index)
        .map(Some)
        .collect::<Vec<_>>();
    indices.insert(drag.placeholder_index.min(len - 1), None);
    indices
}

fn reorder_index(from_index: usize, hovered_index: usize, edge: DropEdge) -> Option<usize> {
    let target = drop_index(from_index, hovered_index, edge);
    (target != from_index).then_some(target)
}

fn drop_index(from_index: usize, hovered_index: usize, edge: DropEdge) -> usize {
    let insertion_slot = hovered_index + usize::from(edge == DropEdge::After);
    insertion_slot.saturating_sub(usize::from(insertion_slot > from_index))
}

#[cfg(test)]
mod tests {
    use super::{ActiveCategoryDrag, DropEdge, drop_index, projected_indices, reorder_index};

    #[test]
    fn projects_placeholder_and_post_removal_drop_indices() {
        assert_eq!(drop_index(0, 2, DropEdge::After), 2);
        assert_eq!(drop_index(2, 0, DropEdge::Before), 0);
        assert_eq!(reorder_index(1, 1, DropEdge::Before), None);
        assert_eq!(
            projected_indices(
                4,
                ActiveCategoryDrag {
                    from_index: 0,
                    placeholder_index: 2,
                }
            ),
            [Some(1), Some(2), None, Some(3)]
        );
    }
}
