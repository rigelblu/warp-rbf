use std::collections::HashSet;
use std::{fmt, iter, mem};

use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2F;
use warp_core::features::FeatureFlag;
use warpui::elements::{
    ChildAnchor, ConstrainedBox, Container, CrossAxisAlignment, DispatchEventResult, Element,
    Empty, EventHandler, Flex, Hoverable, MainAxisAlignment, MainAxisSize, MouseStateHandle,
    OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds, PositionedElementAnchor,
    PositionedElementOffsetBounds, Rect, SavePosition, Shrinkable, Stack,
};
use warpui::platform::Cursor;
use warpui::{AppContext, EntityId, ViewContext};

use super::{ActivationReason, PaneGroup, PaneId};
use crate::app_state;
use crate::pane_group::{get_minimum_pane_size, DraggedBorder, PaneGroupAction};
use crate::themes::theme::WarpTheme;
use crate::ui_components::icons::Icon;

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;

pub(in crate::pane_group) const DEFAULT_FLEX_VALUE: f32 = 1.0;
pub(in crate::pane_group) const DEFAULT_FLEX_SIZE: PaneFlex = PaneFlex(DEFAULT_FLEX_VALUE);

pub fn get_divider_thickness() -> f32 {
    if FeatureFlag::MinimalistUI.is_enabled() {
        1.0
    } else {
        2.0
    }
}

// Extra padding for the divider to make it easier to resize.
// This is added around each side of the divider. Only used
// when minimalist UI is enabled.
const DIVIDER_RESIZE_PADDING: f32 = 4.0;

// #warp-03 — a collapsed pane renders as a thin rail of this thickness (px),
// with an expand chevron of this size centered in it.
const RAIL_THICKNESS: f32 = 20.0;
const RAIL_CHEVRON_SIZE: f32 = 14.0;

/// Tree for all of the split panes
///
/// Holds the root node and maintains the size of the tree
///
/// Also has an option hidden pane id, if you ever want a pane
/// to remain in the tree but not be rendered, which is needed
/// for pane drag and dropping
pub struct PaneData {
    pub root: PaneNode,
    len: usize,
    hidden_panes: Vec<HiddenPane>,
}

#[derive(Debug, Clone, Copy)]
pub struct HiddenPane {
    pub pane_id: PaneId,
    reason: HiddenPaneReason,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HiddenPaneReason {
    FromMove,
    FromJob,
    TemporaryReplacement(PaneId),

    // Pane was closed. We keep it around temporarily in case
    // the user wants to undo the close.
    Closed,

    // Pane is a child agent spawned by an orchestrator. It stays hidden
    // until the user explicitly reveals it from the status card.
    ChildAgent,

    // Pane is collapsed to a thin edge rail (#warp-03). Unlike the other
    // reasons, a collapsed pane is still rendered — as a rail in its slot —
    // but it is excluded from navigation (visible_pane_ids). The render loop
    // (Branch::render) special-cases it; see pane_collapsed.
    Collapsed,
}

impl HiddenPane {
    pub fn from_move(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            reason: HiddenPaneReason::FromMove,
        }
    }
    pub fn from_job(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            reason: HiddenPaneReason::FromJob,
        }
    }
    pub fn from_temporary_replacement(pane_id: PaneId, replacement_pane_id: PaneId) -> Self {
        Self {
            pane_id,
            reason: HiddenPaneReason::TemporaryReplacement(replacement_pane_id),
        }
    }
    pub fn from_close(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            reason: HiddenPaneReason::Closed,
        }
    }
    pub fn from_child_agent(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            reason: HiddenPaneReason::ChildAgent,
        }
    }

    pub fn from_collapse(pane_id: PaneId) -> Self {
        Self {
            pane_id,
            reason: HiddenPaneReason::Collapsed,
        }
    }
}

/// Single Node in the tree of panes
pub enum PaneNode {
    /// A collection of panes split in a specific direction
    Branch(PaneBranch),
    /// A single pane
    Leaf(PaneId),
}

#[derive(Debug)]
pub struct PaneFlex(pub f32);

impl Default for PaneFlex {
    fn default() -> Self {
        PaneFlex(DEFAULT_FLEX_VALUE)
    }
}

impl From<app_state::PaneFlex> for PaneFlex {
    fn from(pane_flex: app_state::PaneFlex) -> Self {
        PaneFlex(pane_flex.0)
    }
}

pub struct PaneBranch {
    axis: SplitDirection,
    pub nodes: Vec<(PaneFlex, PaneNode)>,
    dividers: Vec<Divider>,
}

/// The result of attempting to remove a pane from a branch
enum BranchRemoveResult {
    /// The pane was not found in this sub-tree
    NotFound,
    /// The pane was found and removed, no further action is needed
    Removed,
    /// The pane was found and removed, leaving only a single node in the branch, so it needs to
    /// be collapsed into the parent
    Collapse(PaneNode),
}

/// The result of attempting to find a pane with direction
#[derive(Debug, PartialEq)]
enum FindPaneByDirectionResult {
    /// Located the current pane in the tree.
    Located,
    /// The current pane is not found in the tree.
    NotFound,
    /// A list of possible target panes were found.
    Found(HashSet<PaneId>),
}

trait FindPaneByDirection {
    fn panes_by_direction(
        &self,
        content: PaneId,
        direction: Direction,
    ) -> FindPaneByDirectionResult;
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn axis(&self) -> SplitDirection {
        match self {
            Direction::Left | Direction::Right => SplitDirection::Horizontal,
            Direction::Up | Direction::Down => SplitDirection::Vertical,
        }
    }

    /// The reverse direction. Used by collapse-to-rail's "retract-wins"
    /// restore: pressing the opposite arrow restores the pane your edge
    /// would retreat toward (#warp-03).
    pub(crate) fn opposite(&self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

pub struct Divider {
    id: EntityId,
    mouse_state: MouseStateHandle,
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl Divider {
    pub fn new() -> Self {
        Self {
            id: EntityId::new(),
            mouse_state: Default::default(),
        }
    }
}

impl PaneData {
    /// Create a new `PaneData` with a Leaf as the root
    pub fn new(pane_id: PaneId) -> Self {
        Self {
            root: PaneNode::Leaf(pane_id),
            len: 1,
            hidden_panes: Vec::new(),
        }
    }

    pub fn visible_pane_count(&self) -> usize {
        // Use `visible_pane_ids` directly; subtracting hidden count would
        // double-count temporary-replacement originals (hidden but off-tree).
        self.visible_pane_ids().len()
    }

    pub fn has_horizontal_split(&self) -> bool {
        self.root.has_horizontal_split(&self.hidden_panes)
    }

    pub fn num_hidden_panes(&self) -> usize {
        self.hidden_panes.len()
    }

    pub fn remove_hidden_pane(&mut self, pane_id: PaneId) {
        self.hidden_panes.retain(|pane| pane.pane_id != pane_id);
    }

    /// Create a new `PaneData` with a Branch as the root
    ///
    /// Note: If there is only a single top-level Node (e.g. from a manually edited launch
    /// configuration), then this will collapse that Node into the root of this `PaneData`
    pub fn new_branch(axis: SplitDirection, nodes: Vec<(PaneFlex, PaneNode)>, len: usize) -> Self {
        let root = if nodes.len() == 1 {
            let mut mutable_nodes = nodes;
            // Safety: We know there is exactly one node in the list
            mutable_nodes.pop().unwrap().1
        } else {
            let dividers = iter::repeat_with(Divider::new)
                .take(nodes.len() - 1)
                .collect();
            PaneNode::Branch(PaneBranch {
                axis,
                nodes,
                dividers,
            })
        };

        Self {
            root,
            len,
            hidden_panes: Vec::new(),
        }
    }

    pub fn move_pane(&mut self, id: PaneId, target_pane_id: PaneId, direction: Direction) -> bool {
        if id == target_pane_id {
            return false;
        }

        // If the given move would not result in the pane tree being mutated, just return early
        if self.sibling_by_direction(target_pane_id, direction) == Some(id) {
            return false;
        }

        // Remove the pane from the tree
        if !self.remove(id) {
            log::error!("Pane not found");
            return false;
        }

        // Call a new split to move the pane to the new location
        self.split(target_pane_id, id, direction)
    }

    pub fn hide_pane_for_move(&mut self, id: PaneId) {
        self.hidden_panes.push(HiddenPane::from_move(id));
    }

    pub fn clear_hidden_panes_from_move(&mut self) {
        self.hidden_panes
            .retain(|pane| pane.reason != HiddenPaneReason::FromMove);
    }

    pub fn hide_pane_for_job(&mut self, id: PaneId) {
        self.hidden_panes.push(HiddenPane::from_job(id));
    }

    pub fn show_pane_for_job(&mut self, id: PaneId) {
        if let Some(pos) = self
            .hidden_panes
            .iter()
            .position(|pane| pane.pane_id == id && pane.reason == HiddenPaneReason::FromJob)
        {
            self.hidden_panes.remove(pos);
        } else {
            log::error!("Attempted to show pane for the job but couldn't find it.")
        }
    }

    pub fn hide_pane_for_child_agent(&mut self, id: PaneId) {
        if !self.is_pane_hidden(&id) {
            self.hidden_panes.push(HiddenPane::from_child_agent(id));
        }
    }

    pub fn show_pane_for_child_agent(&mut self, id: PaneId) {
        if let Some(pos) = self
            .hidden_panes
            .iter()
            .position(|pane| pane.pane_id == id && pane.reason == HiddenPaneReason::ChildAgent)
        {
            self.hidden_panes.remove(pos);
        } else {
            log::error!("Attempted to show child agent pane but couldn't find it.")
        }
    }

    /// Returns true if `id` is hidden as a child agent pane.
    pub fn is_pane_hidden_for_child_agent(&self, id: PaneId) -> bool {
        pane_hidden_for_child_agent(&self.hidden_panes, &id)
    }

    /// Collapse a pane to a thin edge rail (#warp-03): it stays in the tree
    /// (still in `pane_ids`) but is excluded from `visible_pane_ids` and is
    /// rendered as a rail in its slot. No-op (returns false) if the pane is
    /// already hidden, or if it is the last visible pane — a tab must always
    /// keep at least one visible pane.
    pub fn collapse_pane(&mut self, id: PaneId) -> bool {
        if self.is_pane_hidden(&id) || self.visible_pane_ids().len() <= 1 {
            return false;
        }
        self.hidden_panes.push(HiddenPane::from_collapse(id));
        true
    }

    /// Restore a collapsed pane to its original position and size. The pane
    /// never left the tree, so this just drops the collapse marker. Returns
    /// false if the pane was not collapsed.
    pub fn restore_collapsed_pane(&mut self, id: PaneId) -> bool {
        if let Some(pos) = self
            .hidden_panes
            .iter()
            .position(|pane| pane.pane_id == id && pane.reason == HiddenPaneReason::Collapsed)
        {
            self.hidden_panes.remove(pos);
            true
        } else {
            false
        }
    }

    /// The currently-collapsed (railed) panes, in collapse order.
    pub fn collapsed_pane_ids(&self) -> Vec<PaneId> {
        self.hidden_panes
            .iter()
            .filter(|hidden| hidden.reason == HiddenPaneReason::Collapsed)
            .map(|hidden| hidden.pane_id)
            .collect()
    }

    pub fn is_pane_collapsed(&self, id: &PaneId) -> bool {
        pane_collapsed(&self.hidden_panes, id)
    }

    pub fn toggle_pane_visibility_for_job(&mut self, id: PaneId) -> bool {
        if pane_hidden_for_job(&self.hidden_panes, &id) {
            self.show_pane_for_job(id);
            true
        } else {
            self.hide_pane_for_job(id);
            false
        }
    }

    pub fn hide_closed_pane(&mut self, id: PaneId) {
        self.hidden_panes.push(HiddenPane::from_close(id));
    }

    pub fn unhide_closed_pane(&mut self, id: PaneId) -> bool {
        if let Some(pos) = self
            .hidden_panes
            .iter()
            .position(|pane| pane.pane_id == id && pane.reason == HiddenPaneReason::Closed)
        {
            self.hidden_panes.remove(pos);
            true
        } else {
            log::warn!(
                "Attempted to show pane {id} for undo close but couldn't find it in hidden panes"
            );
            false
        }
    }

    pub fn get_closed_pane_ids(&self) -> Vec<PaneId> {
        self.hidden_panes
            .iter()
            .filter(|hidden| matches!(hidden.reason, HiddenPaneReason::Closed))
            .map(|hidden| hidden.pane_id)
            .collect()
    }

    pub fn clear_hidden_closed_panes(&mut self) {
        self.hidden_panes
            .retain(|pane| pane.reason != HiddenPaneReason::Closed);
    }

    pub fn is_temporary_replacement(&self, replacement_pane_id: PaneId) -> bool {
        self.original_pane_for_replacement(replacement_pane_id)
            .is_some()
    }

    pub fn original_pane_for_replacement(&self, replacement_pane_id: PaneId) -> Option<PaneId> {
        self.hidden_panes.iter().find_map(|hidden_pane| {
            matches!(hidden_pane.reason, HiddenPaneReason::TemporaryReplacement(id) if id == replacement_pane_id)
                .then_some(hidden_pane.pane_id)
        })
    }

    /// Inverse of [`Self::original_pane_for_replacement`]: given a pane
    /// currently swapped out as a temporary replacement's original,
    /// return the replacement that took its slot.
    pub fn replacement_pane_for_original(&self, original_pane_id: PaneId) -> Option<PaneId> {
        self.hidden_panes.iter().find_map(|hidden_pane| {
            if hidden_pane.pane_id != original_pane_id {
                return None;
            }
            match hidden_pane.reason {
                HiddenPaneReason::TemporaryReplacement(replacement_id) => Some(replacement_id),
                _ => None,
            }
        })
    }

    pub fn is_hidden_closed_pane(&self, pane_id: &PaneId) -> bool {
        self.hidden_panes.iter().any(|hidden_pane| {
            hidden_pane.pane_id == *pane_id && hidden_pane.reason == HiddenPaneReason::Closed
        })
    }

    /// Returns true when a hidden pane should be omitted from app-state snapshots.
    ///
    /// Collapsed panes are intentionally excluded from navigation but still
    /// represent live layout content, so they must snapshot like ordinary panes
    /// unless/until collapse state gets its own persisted representation.
    pub fn should_omit_pane_from_snapshot(&self, pane_id: &PaneId) -> bool {
        self.hidden_panes.iter().any(|hidden_pane| {
            hidden_pane.pane_id == *pane_id && hidden_pane.reason != HiddenPaneReason::Collapsed
        })
    }

    pub fn replace_pane(
        &mut self,
        original_pane_id: PaneId,
        replacement_pane_id: PaneId,
        is_temporary: bool,
    ) -> bool {
        // First, check if the original pane exists in the tree
        if !self.root.contains_pane(original_pane_id) {
            return false;
        }

        // Hide the original pane for temporary replacement
        if is_temporary {
            self.hidden_panes
                .push(HiddenPane::from_temporary_replacement(
                    original_pane_id,
                    replacement_pane_id,
                ));
        }

        // Replace the original pane with the replacement pane in the tree
        let success = self
            .root
            .replace_pane(original_pane_id, replacement_pane_id);

        if success {
            return true;
        } else if is_temporary {
            // If our pane replacement failed, remove the newly added pane from the hidden panes list
            self.hidden_panes.pop();
        }
        false
    }

    pub fn revert_temporary_replacement(&mut self, replacement_pane_id: PaneId) -> Option<PaneId> {
        // Find and remove the hidden pane that corresponds to this replacement
        if let Some(position) = self.hidden_panes.iter().position(|hidden_pane| {
            matches!(hidden_pane.reason, HiddenPaneReason::TemporaryReplacement(id) if id == replacement_pane_id)
        }) {
            let hidden_pane = self.hidden_panes.remove(position);
            let original_pane_id = hidden_pane.pane_id;

            // Replace the replacement pane with the original pane in the tree
            if self.root.replace_pane(replacement_pane_id, original_pane_id) {
                Some(original_pane_id)
            } else {
                // If replacement failed, re-add the hidden pane entry
                self.hidden_panes.insert(position, hidden_pane);
                None
            }
        } else {
            None
        }
    }

    pub fn split(&mut self, old_id: PaneId, new_id: PaneId, direction: Direction) -> bool {
        let successful_split = self.root.split(old_id, new_id, direction);

        if successful_split {
            self.len += 1;
        }

        successful_split
    }

    /// Split the root of the pane tree, inserting `new_id` according to the given direction.
    pub fn split_root(&mut self, new_id: PaneId, direction: Direction) {
        self.root.insert(new_id, direction);
        self.len += 1;
    }

    pub fn remove(&mut self, content: PaneId) -> bool {
        let successful_remove = self.root.remove(content);

        if successful_remove {
            self.len = self.len.saturating_sub(1);
        }

        successful_remove
    }

    /// Get the child panes in an array sorted from left to right, up to down.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        self.root.pane_ids()
    }

    /// Get only the visible child panes in an array sorted from left to right, up to down.
    /// This filters out panes that are hidden for any reason (move, job, close, etc.).
    pub fn visible_pane_ids(&self) -> Vec<PaneId> {
        self.root
            .pane_ids()
            .into_iter()
            .filter(|pane_id| !self.is_pane_hidden(pane_id))
            .collect()
    }

    /// Returns true if the given pane is hidden for any reason.
    pub fn is_pane_hidden(&self, pane_id: &PaneId) -> bool {
        self.hidden_panes
            .iter()
            .any(|hidden_pane| hidden_pane.pane_id == *pane_id)
    }

    /// Returns true if `pane_id` is currently a leaf in the layout tree.
    pub fn is_pane_in_tree(&self, pane_id: PaneId) -> bool {
        self.root.contains_pane(pane_id)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn render(&self, theme: &WarpTheme, app: &AppContext) -> Box<dyn Element> {
        match &self.root {
            PaneNode::Leaf(pane) => pane.render(app),
            PaneNode::Branch(node) => node.render(theme, &self.hidden_panes, app),
        }
    }

    pub fn adjust_pane_size(
        &mut self,
        border_id: EntityId,
        delta: f32,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let hidden_panes = self.hidden_panes.clone();
        self.root
            .adjust_pane_size(border_id, delta, &hidden_panes, ctx);
    }

    pub fn reset_pane_sizes(&mut self, border_id: EntityId) -> bool {
        self.root.reset_pane_sizes(border_id)
    }

    pub fn distribute_pane_sizes(&mut self, axis: SplitDirection) -> bool {
        self.root.distribute_pane_sizes(axis, &self.hidden_panes)
    }

    pub fn adjust_pane_size_by_id(
        &mut self,
        pane_id: PaneId,
        direction: SplitDirection,
        delta: f32,
        ctx: &mut ViewContext<PaneGroup>,
    ) {
        let hidden_panes = self.hidden_panes.clone();
        self.root
            .adjust_pane_size_by_id(pane_id, direction, delta, &hidden_panes, ctx);
    }

    pub fn panes_by_direction(
        &self,
        pane_id: PaneId,
        direction: Direction,
        ctx: &ViewContext<PaneGroup>,
    ) -> Vec<PaneId> {
        // Find the panes from the current pane in the given direction.
        // Due to uneven splits, we may have multiple panes in the same direction.
        // Detect which ones are touching the current pane by checking the boundaries from the view context.

        if let FindPaneByDirectionResult::Found(ids) =
            self.root.panes_by_direction(pane_id, direction)
        {
            if let Some(current_rect) = ctx.element_position_by_id(pane_id.position_id()) {
                ids.into_iter()
                    .filter(|id| {
                        match ctx.element_position_by_id(id.position_id()) {
                            Some(candidate_rect) => PaneData::are_rects_overlapping(
                                &current_rect,
                                &candidate_rect,
                                direction.axis(),
                            ),
                            None => true, // If we can't find the position, we assume it's overlapping
                        }
                    })
                    .collect()
            } else {
                Vec::from_iter(ids)
            }
        } else {
            // We didn't find any panes in that direction, return an empty list
            Vec::new()
        }
    }

    /// #warp-03 — the structural sibling subtree bordering `pane_id` in
    /// `direction`: walk to the ancestor branch whose split runs along the
    /// direction and return that adjacent slot's pane ids — a single pane in a
    /// flat split, a whole bordering column/row when the slot is itself a
    /// branch. Unlike `panes_by_direction` (geometry-filtered, for focus-nav),
    /// this is purely structural; it's what "rail the whole bordering group"
    /// routes through. Empty when `pane_id` is at the tree's edge in `direction`.
    pub fn pane_group_by_direction(&self, pane_id: PaneId, direction: Direction) -> Vec<PaneId> {
        match &self.root {
            PaneNode::Branch(branch) => branch
                .pane_group_by_direction(pane_id, direction)
                .unwrap_or_default(),
            PaneNode::Leaf(_) => Vec::new(),
        }
    }

    fn are_rects_overlapping(rect1: &RectF, rect2: &RectF, axis: SplitDirection) -> bool {
        // Returns true if the two rectangles overlap in the given axis.
        //
        //                 ---------
        //  -----------    | rect2 |
        // | rect1     |   ---------
        // |           |
        //  -----------
        //
        // In this case, the function would return true for SplitDirection::Horizontal.
        // It would return false for SplitDirection::Vertical.
        match axis {
            SplitDirection::Horizontal => {
                !(rect1.max_y() <= rect2.min_y() || rect1.min_y() >= rect2.max_y())
            }
            SplitDirection::Vertical => {
                !(rect1.max_x() <= rect2.min_x() || rect1.min_x() >= rect2.max_x())
            }
        }
    }

    // Find a pane from the given pane in the given direction, but only if it is a direct sibling
    //  of the given pane.  This means they are direct children of the same branch.
    fn sibling_by_direction(&self, pane_id: PaneId, direction: Direction) -> Option<PaneId> {
        match &self.root {
            PaneNode::Branch(b) => b.sibling_by_direction(pane_id, direction),
            _ => None,
        }
    }
}

impl PaneNode {
    fn has_visible_children(&self, hidden_panes: &[HiddenPane]) -> bool {
        match self {
            PaneNode::Leaf(pane_id) => {
                !pane_hidden_for_job(hidden_panes, pane_id)
                    && !pane_hidden_for_undo(hidden_panes, pane_id)
                    && !pane_hidden_for_move(hidden_panes, pane_id)
                    && !pane_hidden_for_child_agent(hidden_panes, pane_id)
            }
            PaneNode::Branch(branch) => branch.has_visible_children(hidden_panes),
        }
    }

    fn has_children_hidden_for_move(&self, hidden_panes: &[HiddenPane]) -> bool {
        match self {
            PaneNode::Leaf(pane_id) => pane_hidden_for_move(hidden_panes, pane_id),
            PaneNode::Branch(branch) => branch.has_children_hidden_for_move(hidden_panes),
        }
    }

    pub fn has_horizontal_split(&self, hidden_panes: &[HiddenPane]) -> bool {
        match self {
            PaneNode::Leaf(_) => false,
            PaneNode::Branch(branch) => {
                let mut visible_or_move_children = 0usize;
                let mut any_child_split = false;

                for (_, child) in &branch.nodes {
                    if !child.has_visible_children(hidden_panes)
                        && !child.has_children_hidden_for_move(hidden_panes)
                    {
                        continue;
                    }

                    visible_or_move_children += 1;

                    if child.has_horizontal_split(hidden_panes) {
                        any_child_split = true;
                    }
                }

                let self_has_split =
                    branch.axis == SplitDirection::Horizontal && visible_or_move_children > 1;

                self_has_split || any_child_split
            }
        }
    }

    fn split(&mut self, old_pane_id: PaneId, new_pane_id: PaneId, direction: Direction) -> bool {
        match self {
            PaneNode::Leaf(pane) => {
                if *pane == old_pane_id {
                    *self = PaneNode::Branch(PaneBranch::for_leaves(
                        old_pane_id,
                        new_pane_id,
                        direction,
                    ));
                    true
                } else {
                    false
                }
            }
            PaneNode::Branch(branch) => branch.split(old_pane_id, new_pane_id, direction),
        }
    }

    /// Number of splits at the node in the given axis. For leaf nodes, this is always one.
    pub fn num_splits_in_direction(&self, axis: SplitDirection) -> usize {
        match self {
            PaneNode::Branch(branch) if branch.axis == axis => branch.nodes.len(),
            _ => 1,
        }
    }

    fn remove(&mut self, pane_id: PaneId) -> bool {
        match self {
            // Leaves can only be removed from the containing branch
            PaneNode::Leaf(_) => false,
            PaneNode::Branch(branch) => match branch.remove(pane_id) {
                BranchRemoveResult::NotFound => false,
                BranchRemoveResult::Removed => true,
                BranchRemoveResult::Collapse(last_node) => {
                    *self = last_node;
                    true
                }
            },
        }
    }

    fn insert(&mut self, new_pane_id: PaneId, direction: Direction) {
        match self {
            PaneNode::Leaf(old_pane_id) => {
                *self =
                    PaneNode::Branch(PaneBranch::for_leaves(*old_pane_id, new_pane_id, direction));
            }
            PaneNode::Branch(branch) => branch.insert(new_pane_id, direction),
        }
    }

    fn pane_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(pane) => vec![*pane],
            PaneNode::Branch(branch) => branch.get_children(),
        }
    }

    fn render(
        &self,
        theme: &WarpTheme,
        hidden_panes: &Vec<HiddenPane>,
        app: &AppContext,
    ) -> Box<dyn Element> {
        match self {
            PaneNode::Leaf(view) => {
                let view = *view;
                EventHandler::new(view.render(app))
                    .on_left_mouse_down(move |ctx, _, _| {
                        ctx.dispatch_typed_action(PaneGroupAction::Activate(
                            view,
                            ActivationReason::Click,
                        ));
                        DispatchEventResult::StopPropagation
                    })
                    .finish()
            }
            PaneNode::Branch(branch) => branch.render(theme, hidden_panes, app),
        }
    }

    pub fn pane_size(&self, ctx: &mut ViewContext<PaneGroup>) -> Vector2F {
        match self {
            PaneNode::Leaf(pane) => ctx
                .element_position_by_id(pane.position_id())
                .map_or(Vector2F::zero(), |rect| rect.size()),
            PaneNode::Branch(branch) => branch.size(ctx),
        }
    }

    pub fn adjust_pane_size(
        &mut self,
        border_id: EntityId,
        delta: f32,
        hidden_panes: &[HiddenPane],
        ctx: &mut ViewContext<PaneGroup>,
    ) -> bool {
        match self {
            PaneNode::Leaf(_) => false,
            PaneNode::Branch(branch) => {
                branch.adjust_pane_size(border_id, delta, hidden_panes, ctx)
            }
        }
    }

    pub fn reset_pane_sizes(&mut self, border_id: EntityId) -> bool {
        match self {
            PaneNode::Leaf(_) => false,
            PaneNode::Branch(branch) => branch.reset_pane_sizes(border_id),
        }
    }

    fn distribute_pane_sizes(&mut self, axis: SplitDirection, hidden_panes: &[HiddenPane]) -> bool {
        match self {
            PaneNode::Leaf(_) => false,
            PaneNode::Branch(branch) => branch.distribute_pane_sizes(axis, hidden_panes),
        }
    }

    /// The boolean value returned here indicates whether a resizing needs to
    /// be handled at a parent branch. For a leaf node, if the pane's id does not match,
    /// we returns false as its parent branch does not need to handle the resize.
    /// If it does match, we returns true so its parent branch will handle it.
    /// For a branch node, if the direction we are resizing does not match the branch
    /// axis, it will return true so a parent branch that does match will handle the
    /// resize.
    pub fn adjust_pane_size_by_id(
        &mut self,
        pane_id: PaneId,
        direction: SplitDirection,
        delta: f32,
        hidden_panes: &[HiddenPane],
        ctx: &mut ViewContext<PaneGroup>,
    ) -> bool {
        match self {
            PaneNode::Leaf(id) => *id == pane_id,
            PaneNode::Branch(branch) => {
                branch.adjust_pane_size_by_id(pane_id, direction, delta, hidden_panes, ctx)
            }
        }
    }

    /// Find the first panes in the given direction inside of this pane.
    fn first_panes_in_direction(&self, direction: Direction) -> HashSet<PaneId> {
        match self {
            // If this is a leaf, then this is the first pane from any direction.
            PaneNode::Leaf(id) => HashSet::from([*id]),
            PaneNode::Branch(branch) => {
                // If the direction matches the split axis, then we only search the first sub-tree in the given direction.
                //  --------------------     The first panes from the left are 1 and 3.
                //  |   1     |    2   |     The first panes from the right are 2 and 3.
                //  --------------------     For these cases we must search both sub-trees.
                //  |        3         |
                //  --------------------     The first pane from down is 3.  We only need to search the first sub-tree.
                if branch.axis() == direction.axis() {
                    match direction {
                        Direction::Left | Direction::Up => branch
                            .nodes
                            .last()
                            .expect("PaneGroup has no nodes when moving focus.")
                            .1
                            .first_panes_in_direction(direction),
                        Direction::Right | Direction::Down => branch
                            .nodes
                            .first()
                            .expect("PaneBranch has no nodes when moving focus.")
                            .1
                            .first_panes_in_direction(direction),
                    }
                } else {
                    branch
                        .nodes
                        .iter()
                        .flat_map(|(_, node)| node.first_panes_in_direction(direction))
                        .collect()
                }
            }
        }
    }

    #[cfg(test)]
    fn as_branch(&self) -> Option<&PaneBranch> {
        match self {
            PaneNode::Branch(branch) => Some(branch),
            PaneNode::Leaf(_) => None,
        }
    }

    #[cfg(test)]
    fn as_leaf(&self) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(id) => Some(*id),
            PaneNode::Branch(_) => None,
        }
    }

    /// Sum this [`PaneNode`]s [`PaneFlex`] values along the given `axis`. Return the
    /// [`DEFAULT_FLEX_SIZE`] if this [`PaneNode`] isn't a [`PaneNode::Branch`] in the given
    /// [`SplitDirection`] (or it is a [`PaneNode::Leaf`]).
    pub(in crate::pane_group) fn pane_flex_sum_along_axis(&self, axis: SplitDirection) -> f32 {
        match self {
            PaneNode::Branch(pane_branch) if pane_branch.axis == axis => pane_branch
                .nodes
                .iter()
                .fold(0., |sum, (pane_flex, _)| sum + pane_flex.0),
            _ => DEFAULT_FLEX_VALUE,
        }
    }

    pub(crate) fn contains_pane(&self, pane_id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(id) => *id == pane_id,
            PaneNode::Branch(branch) => branch.contains_pane(pane_id),
        }
    }

    fn replace_pane(&mut self, old_pane_id: PaneId, new_pane_id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(id) => {
                if *id == old_pane_id {
                    *id = new_pane_id;
                    true
                } else {
                    false
                }
            }
            PaneNode::Branch(branch) => branch.replace_pane(old_pane_id, new_pane_id),
        }
    }
}

impl FindPaneByDirection for PaneNode {
    fn panes_by_direction(
        &self,
        pane_id: PaneId,
        direction: Direction,
    ) -> FindPaneByDirectionResult {
        match self {
            PaneNode::Leaf(id) => {
                if *id == pane_id {
                    FindPaneByDirectionResult::Located
                } else {
                    FindPaneByDirectionResult::NotFound
                }
            }
            PaneNode::Branch(branch) => branch.panes_by_direction(pane_id, direction),
        }
    }
}

impl PaneBranch {
    fn new(old_pane: PaneNode, new_pane: PaneNode, direction: Direction) -> Self {
        let axis = direction.axis();
        PaneBranch {
            axis,
            nodes: match direction {
                Direction::Left | Direction::Up => {
                    vec![(DEFAULT_FLEX_SIZE, new_pane), (DEFAULT_FLEX_SIZE, old_pane)]
                }
                Direction::Right | Direction::Down => {
                    vec![(DEFAULT_FLEX_SIZE, old_pane), (DEFAULT_FLEX_SIZE, new_pane)]
                }
            },
            dividers: vec![Divider::new()],
        }
    }

    /// Construct a branch that contains two leaves.
    fn for_leaves(old_leaf: PaneId, new_leaf: PaneId, direction: Direction) -> Self {
        Self::new(
            PaneNode::Leaf(old_leaf),
            PaneNode::Leaf(new_leaf),
            direction,
        )
    }

    fn split(&mut self, old_pane: PaneId, new_pane: PaneId, direction: Direction) -> bool {
        for (idx, (_, node)) in self.nodes.iter_mut().enumerate() {
            match node {
                PaneNode::Branch(branch) => {
                    if branch.split(old_pane, new_pane, direction) {
                        return true;
                    }
                }
                PaneNode::Leaf(pane) => {
                    if *pane == old_pane {
                        // If the split comes in the same direction as the previous splits
                        // on this sub-tree, we can insert the new pane into the nodes directly
                        if direction.axis() == self.axis {
                            self.nodes.insert(
                                match direction {
                                    Direction::Left | Direction::Up => idx,
                                    Direction::Right | Direction::Down => idx + 1,
                                },
                                (DEFAULT_FLEX_SIZE, PaneNode::Leaf(new_pane)),
                            );
                            self.dividers.insert(idx, Divider::new());
                        } else {
                            // Otherwise, split the current leaf into a perpendicular branch
                            *node = PaneNode::Branch(PaneBranch::for_leaves(
                                *pane, new_pane, direction,
                            ));
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Inserts `new_pane_id` into this branch at either the start or the end, according to the
    /// [`Direction`]. If the direction axis does not match that of this branch, the branch is
    /// re-split in place.
    fn insert(&mut self, new_pane_id: PaneId, direction: Direction) {
        if direction.axis() == self.axis {
            match direction {
                Direction::Left | Direction::Up => {
                    self.nodes
                        .insert(0, (DEFAULT_FLEX_SIZE, PaneNode::Leaf(new_pane_id)));
                    self.dividers.insert(0, Divider::new());
                }
                Direction::Right | Direction::Down => {
                    self.nodes
                        .push((DEFAULT_FLEX_SIZE, PaneNode::Leaf(new_pane_id)));
                    self.dividers.push(Divider::new());
                }
            }
        } else {
            // If the axes don't match, split this branch in place.
            let nodes = mem::take(&mut self.nodes);
            let dividers = mem::take(&mut self.dividers);
            let axis = self.axis;
            *self = PaneBranch::new(
                PaneNode::Branch(PaneBranch {
                    nodes,
                    dividers,
                    axis,
                }),
                PaneNode::Leaf(new_pane_id),
                direction,
            );
        }
    }

    fn remove(&mut self, pane_id_to_remove: PaneId) -> BranchRemoveResult {
        for (idx, (_, node)) in self.nodes.iter_mut().enumerate() {
            match node {
                PaneNode::Branch(_) => {
                    if node.remove(pane_id_to_remove) {
                        return BranchRemoveResult::Removed;
                    }
                }
                PaneNode::Leaf(pane) => {
                    if *pane == pane_id_to_remove {
                        self.nodes.remove(idx);
                        if self.dividers.is_empty() {
                            log::error!("Attempted to remove a pane when there are no dividers!");
                        } else {
                            self.dividers.remove(idx.min(self.dividers.len() - 1));
                        }
                        if self.nodes.len() == 1 {
                            // Safety: We know that there is an element in `self.nodes`
                            return BranchRemoveResult::Collapse(self.nodes.pop().unwrap().1);
                        } else {
                            return BranchRemoveResult::Removed;
                        }
                    }
                }
            }
        }

        BranchRemoveResult::NotFound
    }

    fn get_children(&self) -> Vec<PaneId> {
        let mut res = vec![];
        for (_, member) in &self.nodes {
            match member {
                PaneNode::Branch(branch) => res.extend(branch.get_children()),
                PaneNode::Leaf(leaf) => res.push(*leaf),
            }
        }
        res
    }

    /// Returns the leaf panes that are direct children of this branch.
    #[cfg(test)]
    fn direct_children(&self) -> Vec<PaneId> {
        self.nodes
            .iter()
            .filter_map(|(_, node)| match node {
                PaneNode::Leaf(id) => Some(*id),
                PaneNode::Branch(_) => None,
            })
            .collect()
    }

    /// Returns a reference to the child node at `index`, panicking if it's out of bounds.
    #[cfg(test)]
    fn node(&self, index: usize) -> &PaneNode {
        let (_, node) = &self.nodes[index];
        node
    }

    fn render(
        &self,
        theme: &WarpTheme,
        hidden_panes: &Vec<HiddenPane>,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut parent = match self.axis {
            SplitDirection::Horizontal => Flex::row(),
            SplitDirection::Vertical => Flex::column(),
        };

        // Iterate through all the panes, skipping nodes that have no visible children
        // except when children are hidden for move operations (we need empty drop targets)
        let mut dividers = self.dividers.iter();

        // Collect divider positions to render them as positioned elements later.
        let mut divider_positions = Vec::new();

        for (idx, (flex, node)) in self.nodes.iter().enumerate() {
            // Skip nodes that have no visible children, but preserve nodes with children
            // hidden for move operations as they serve as drop targets
            if !node.has_visible_children(hidden_panes)
                && !node.has_children_hidden_for_move(hidden_panes)
            {
                continue;
            }
            // #warp-03: a collapsed leaf — or a fully-collapsed subtree — renders
            // as ONE thin in-place rail (a non-flexible child, so siblings keep
            // their flex and reflow into the freed space). A fully-railed bordering
            // column/row coalesces to a single strip; clicking it restores the
            // whole group. The rail carries no divider; consume this slot's divider
            // so the divider/pane alignment still holds.
            let railed_group = railed_pane_ids(node, hidden_panes);
            if let Some(group) = railed_group {
                parent.add_child(create_rail(self.axis, idx == 0, group, theme));
                dividers.next();
                continue;
            }
            let mut flex_value = flex.0;
            if let PaneNode::Leaf(id) = node {
                // If the pane is hidden for a move, render a divider, but set the
                // child element's flex value to 0 to skip rendering the pane's contents.
                if pane_hidden_for_move(hidden_panes, id) {
                    flex_value = 0.;
                }
            }

            parent.add_child(
                Shrinkable::new(flex_value, node.render(theme, hidden_panes, app)).finish(),
            );
            if let Some(divider) = dividers.next() {
                if matches!(node, PaneNode::Leaf(id) if pane_hidden_for_move(hidden_panes, id)) {
                    continue;
                }
                // #warp-03: a rail has no divider of its own, but the real panes
                // that flank it stay resizable across it — so render this pane's
                // divider whenever a real (non-railed) pane follows, even past
                // one or more rails. Suppress only when nothing real follows (a
                // rail run to the branch's trailing edge): there's nothing to
                // resize against. `adjust_pane_size` resolves the divider to the
                // real pair, skipping the rail(s).
                if self.next_resizable_index(idx, hidden_panes).is_none() {
                    continue;
                }
                // Store a position index to render the actual divider at after we've rendered all pane content.
                // The reason we don't render the actual divider here is that, if we have rich content
                // (or anything that listens for a mouse click) to the right/bottom of the divider,
                // that content is rendered after the divider. Because of that, the clickbox for
                // that content takes precedence over the divider's clickbox, meaning the divider
                // is not clickable when this content is in the blocklist. To fix this, we wait to
                // render the actual divider until after we've rendered all pane content. We
                // cannot use an overlay because content like right click menus that overflow over
                // the divider should still take precedence over the divider's clickbox.
                let position_id = format!("divider_placeholder_{}", divider.id);
                divider_positions.push((divider, position_id.clone()));
                parent.add_child(create_divider_placeholder(self.axis, &position_id));
            }
        }
        let mut stack = Stack::new().with_constrain_absolute_children();
        stack.add_child(parent.finish());

        // Add actual dividers as positioned children anchored to their placeholders
        // (the reason we have to do it this way is explained in the large comment above)
        for (divider, position_id) in divider_positions {
            let divider_element = if FeatureFlag::MinimalistUI.is_enabled() {
                create_minimalist_divider(self.axis, divider, theme)
            } else {
                create_divider(self.axis, divider, theme)
            };

            stack.add_positioned_child(
                divider_element,
                OffsetPositioning::offset_from_save_position_element(
                    position_id,
                    Vector2F::new(0., 0.),
                    PositionedElementOffsetBounds::Unbounded,
                    PositionedElementAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            );
        }
        stack.finish()
    }

    /// #warp-03: the index of the next node after `after` that renders as a
    /// real, resizable pane — skipping rails (collapsed leaves / fully-collapsed
    /// subtrees, which render at a fixed size). `None` when only rails or the
    /// branch's trailing edge follow.
    fn next_resizable_index(&self, after: usize, hidden_panes: &[HiddenPane]) -> Option<usize> {
        (after + 1..self.nodes.len()).find(|&j| !is_railed(&self.nodes[j].1, hidden_panes))
    }

    /// #warp-03: the nearest real, resizable pane before `before`, skipping rails.
    fn prev_resizable_index(&self, before: usize, hidden_panes: &[HiddenPane]) -> Option<usize> {
        (0..before)
            .rev()
            .find(|&j| !is_railed(&self.nodes[j].1, hidden_panes))
    }

    /// #warp-03: the two real panes a divider drag resizes. A rail isn't
    /// resizable and carries no divider, so the divider at `divider_idx` resizes
    /// the real pane that owns it against the nearest real pane beyond it,
    /// leaving any rail(s) in between at their fixed size. With no rail this is
    /// just the adjacent pair `(divider_idx, divider_idx + 1)`. `None` when no
    /// real pane follows.
    fn resize_pair_across_rails(
        &self,
        divider_idx: usize,
        hidden_panes: &[HiddenPane],
    ) -> Option<(usize, usize)> {
        if is_railed(&self.nodes[divider_idx].1, hidden_panes) {
            return None;
        }
        let right = self.next_resizable_index(divider_idx, hidden_panes)?;
        Some((divider_idx, right))
    }

    /// #warp-03: which divider a keyboard resize of the focused pane at
    /// `focused_idx` acts on — its own trailing divider when a real pane follows
    /// (skipping rails), otherwise the nearest real pane's divider on the leading
    /// side (the focused pane is the last real one). `None` when it is the only
    /// real pane in the branch. The returned index is always a valid divider.
    fn keyboard_resize_divider_idx(
        &self,
        focused_idx: usize,
        hidden_panes: &[HiddenPane],
    ) -> Option<usize> {
        if self
            .next_resizable_index(focused_idx, hidden_panes)
            .is_some()
        {
            // A real pane follows, so `focused_idx` is not the last node and its
            // trailing divider exists; `adjust_pane_size` resolves it across rails.
            Some(focused_idx)
        } else {
            self.prev_resizable_index(focused_idx, hidden_panes)
        }
    }

    pub fn adjust_pane_size(
        &mut self,
        border_id: EntityId,
        delta: f32,
        hidden_panes: &[HiddenPane],
        ctx: &mut ViewContext<PaneGroup>,
    ) -> bool {
        if let Some(divider_idx) = self
            .dividers
            .iter()
            .position(|divider| divider.id == border_id)
        {
            // #warp-03: resize the two real panes that flank this divider,
            // skipping any collapsed rail between them (the rail keeps its fixed
            // size). Without a rail this is just the adjacent pair.
            let Some((left, right)) = self.resize_pair_across_rails(divider_idx, hidden_panes)
            else {
                return true;
            };

            let pane_size_1 = self.nodes[left].1.pane_size(ctx);
            let pane_size_2 = self.nodes[right].1.pane_size(ctx);

            let flex_1 = self.nodes[left].0 .0;
            let flex_2 = self.nodes[right].0 .0;

            let total_flex = flex_1 + flex_2;

            let (size_1, size_2) = match self.axis {
                SplitDirection::Horizontal => (pane_size_1.x(), pane_size_2.x()),
                SplitDirection::Vertical => (pane_size_1.y(), pane_size_2.y()),
            };

            // Omit noise in dragging.
            let minimum_pane_size = get_minimum_pane_size(ctx);
            if size_1 + delta < minimum_pane_size
                || size_2 - delta < minimum_pane_size
                || delta.abs() < f32::EPSILON
            {
                return true;
            }

            // Re-distribute the flex factors.
            let new_flex = ((size_1 + delta) / (size_1 + size_2) * total_flex)
                .max(0.)
                .min(total_flex);

            self.nodes[left].0 = PaneFlex(new_flex);
            self.nodes[right].0 = PaneFlex(total_flex - new_flex);

            return true;
        }

        for (_, node) in &mut self.nodes {
            if node.adjust_pane_size(border_id, delta, hidden_panes, ctx) {
                return true;
            }
        }

        false
    }

    pub fn reset_pane_sizes(&mut self, border_id: EntityId) -> bool {
        if self.dividers.iter().any(|divider| divider.id == border_id) {
            for (flex, _) in &mut self.nodes {
                *flex = DEFAULT_FLEX_SIZE;
            }
            return true;
        }

        for (_, node) in &mut self.nodes {
            if node.reset_pane_sizes(border_id) {
                return true;
            }
        }

        false
    }

    fn distribute_pane_sizes(&mut self, axis: SplitDirection, hidden_panes: &[HiddenPane]) -> bool {
        let mut changed = false;

        if self.axis == axis {
            for (flex, node) in &mut self.nodes {
                if railed_pane_ids(node, hidden_panes).is_some() {
                    continue;
                }

                if (flex.0 - DEFAULT_FLEX_VALUE).abs() > f32::EPSILON {
                    *flex = DEFAULT_FLEX_SIZE;
                    changed = true;
                }

                changed |= node.distribute_pane_sizes(axis, hidden_panes);
            }
        } else {
            for (_, node) in &mut self.nodes {
                changed |= node.distribute_pane_sizes(axis, hidden_panes);
            }
        }

        changed
    }

    // Get the size of a branch by recursively adding the size of its children.
    pub fn size(&self, ctx: &mut ViewContext<PaneGroup>) -> Vector2F {
        match self.axis {
            SplitDirection::Horizontal => Vector2F::new(
                self.nodes
                    .iter()
                    .fold(0., |x, (_, node)| x + node.pane_size(ctx).x()),
                self.nodes[0].1.pane_size(ctx).y(),
            ),
            SplitDirection::Vertical => Vector2F::new(
                self.nodes[0].1.pane_size(ctx).x(),
                self.nodes
                    .iter()
                    .fold(0., |y, (_, node)| y + node.pane_size(ctx).y()),
            ),
        }
    }

    pub fn adjust_pane_size_by_id(
        &mut self,
        pane_id: PaneId,
        direction: SplitDirection,
        delta: f32,
        hidden_panes: &[HiddenPane],
        ctx: &mut ViewContext<PaneGroup>,
    ) -> bool {
        for idx in 0..self.nodes.len() {
            if self.nodes[idx].1.adjust_pane_size_by_id(
                pane_id,
                direction,
                delta,
                hidden_panes,
                ctx,
            ) {
                // If the resizing direction is different from the splitting direction
                // of the branch, we return for the parents to handle.
                if direction != self.axis {
                    return true;
                }

                // #warp-03: resize the focused pane against its nearest *real*
                // neighbor, skipping any collapsed rail. The rail previously
                // swallowed the resize at the min-size guard (a 20px rail is
                // always below the minimum pane size).
                if let Some(divider_idx) = self.keyboard_resize_divider_idx(idx, hidden_panes) {
                    let divider_id = self.dividers[divider_idx].id;
                    self.adjust_pane_size(divider_id, delta, hidden_panes, ctx);
                }
                return false;
            }
        }
        false
    }

    pub fn axis(&self) -> SplitDirection {
        self.axis
    }

    // Find the sibling of the given pane in the given direction.
    //  They must be direct children of the same branch.
    fn sibling_by_direction(&self, pane_id: PaneId, direction: Direction) -> Option<PaneId> {
        for (idx, (_, node)) in self.nodes.iter().enumerate() {
            match node {
                PaneNode::Branch(branch) => {
                    if let Some(id) = branch.sibling_by_direction(pane_id, direction) {
                        return Some(id);
                    }
                }
                PaneNode::Leaf(id) => {
                    if direction.axis() == self.axis() && *id == pane_id {
                        return match direction {
                            Direction::Left | Direction::Up => {
                                if idx == 0 {
                                    None
                                } else {
                                    match &self.nodes[idx - 1].1 {
                                        PaneNode::Leaf(id) => Some(*id),
                                        _ => None,
                                    }
                                }
                            }
                            Direction::Right | Direction::Down => {
                                if idx == self.nodes.len() - 1 {
                                    None
                                } else {
                                    match &self.nodes[idx + 1].1 {
                                        PaneNode::Leaf(id) => Some(*id),
                                        _ => None,
                                    }
                                }
                            }
                        };
                    }
                }
            }
        }
        None
    }

    /// See `PaneData::pane_group_by_direction`. Recursive: the innermost branch
    /// whose axis matches `direction` and where `pane_id`'s subtree has an
    /// adjacent sibling in that direction wins, returning that sibling subtree's
    /// pane ids.
    fn pane_group_by_direction(
        &self,
        pane_id: PaneId,
        direction: Direction,
    ) -> Option<Vec<PaneId>> {
        let idx = self
            .nodes
            .iter()
            .position(|(_, node)| node.contains_pane(pane_id))?;

        // Innermost matching branch wins: try the containing child first.
        if let PaneNode::Branch(child) = &self.nodes[idx].1 {
            if let Some(group) = child.pane_group_by_direction(pane_id, direction) {
                return Some(group);
            }
        }

        // Otherwise, if this branch splits along the press direction, the
        // adjacent slot in that direction is the bordering group.
        if direction.axis() == self.axis {
            let adjacent = match direction {
                Direction::Left | Direction::Up => idx.checked_sub(1)?,
                Direction::Right | Direction::Down => {
                    let next = idx + 1;
                    (next < self.nodes.len()).then_some(next)?
                }
            };
            return Some(self.nodes[adjacent].1.pane_ids());
        }

        None
    }

    fn contains_pane(&self, pane_id: PaneId) -> bool {
        self.nodes
            .iter()
            .any(|(_, node)| node.contains_pane(pane_id))
    }

    fn replace_pane(&mut self, old_pane_id: PaneId, new_pane_id: PaneId) -> bool {
        for (_, node) in &mut self.nodes {
            if node.replace_pane(old_pane_id, new_pane_id) {
                return true;
            }
        }
        false
    }

    fn has_visible_children(&self, hidden_panes: &[HiddenPane]) -> bool {
        self.nodes
            .iter()
            .any(|(_, node)| node.has_visible_children(hidden_panes))
    }

    fn has_children_hidden_for_move(&self, hidden_panes: &[HiddenPane]) -> bool {
        self.nodes
            .iter()
            .any(|(_, node)| node.has_children_hidden_for_move(hidden_panes))
    }
}

fn pane_hidden_for_job(hidden_panes: &[HiddenPane], id: &PaneId) -> bool {
    hidden_panes
        .iter()
        .any(|pane| pane.reason == HiddenPaneReason::FromJob && pane.pane_id == *id)
}

fn pane_hidden_for_move(hidden_panes: &[HiddenPane], id: &PaneId) -> bool {
    hidden_panes
        .iter()
        .any(|pane| pane.reason == HiddenPaneReason::FromMove && pane.pane_id == *id)
}

fn pane_hidden_for_undo(hidden_panes: &[HiddenPane], id: &PaneId) -> bool {
    hidden_panes
        .iter()
        .any(|pane| pane.reason == HiddenPaneReason::Closed && pane.pane_id == *id)
}

fn pane_hidden_for_child_agent(hidden_panes: &[HiddenPane], id: &PaneId) -> bool {
    hidden_panes
        .iter()
        .any(|pane| pane.reason == HiddenPaneReason::ChildAgent && pane.pane_id == *id)
}

fn pane_collapsed(hidden_panes: &[HiddenPane], id: &PaneId) -> bool {
    hidden_panes
        .iter()
        .any(|pane| pane.reason == HiddenPaneReason::Collapsed && pane.pane_id == *id)
}

/// #warp-03: the pane ids a node renders as a single rail when it is fully
/// collapsed — a collapsed leaf, or a branch whose every pane is collapsed.
/// `None` when the node has at least one visible (non-railed) pane. Shared by
/// the render loop and the resize path so "what is a rail" has one definition.
fn railed_pane_ids(node: &PaneNode, hidden_panes: &[HiddenPane]) -> Option<Vec<PaneId>> {
    match node {
        PaneNode::Leaf(id) if pane_collapsed(hidden_panes, id) => Some(vec![*id]),
        PaneNode::Branch(_) => {
            let ids = node.pane_ids();
            (!ids.is_empty() && ids.iter().all(|id| pane_collapsed(hidden_panes, id)))
                .then_some(ids)
        }
        _ => None,
    }
}

/// #warp-03: whether a node renders as a rail (a fixed-size, non-resizable
/// strip) rather than a real pane. Resize skips these.
fn is_railed(node: &PaneNode, hidden_panes: &[HiddenPane]) -> bool {
    railed_pane_ids(node, hidden_panes).is_some()
}

impl FindPaneByDirection for PaneBranch {
    fn panes_by_direction(
        &self,
        pane_id: PaneId,
        direction: Direction,
    ) -> FindPaneByDirectionResult {
        for (idx, (_, node)) in self.nodes.iter().enumerate() {
            let res = node.panes_by_direction(pane_id, direction);

            match res {
                FindPaneByDirectionResult::Found(_) => return res,
                FindPaneByDirectionResult::Located => {
                    // If the axis is different, we left for the parent branch to handle.
                    if direction.axis() != self.axis {
                        return res;
                    }

                    let target_panes = match direction {
                        Direction::Left | Direction::Up => {
                            if idx == 0 {
                                return res;
                            }
                            self.nodes[idx - 1].1.first_panes_in_direction(direction)
                        }
                        Direction::Right | Direction::Down => {
                            if idx == self.nodes.len() - 1 {
                                return res;
                            }
                            self.nodes[idx + 1].1.first_panes_in_direction(direction)
                        }
                    };

                    return FindPaneByDirectionResult::Found(target_panes);
                }
                FindPaneByDirectionResult::NotFound => (),
            }
        }
        FindPaneByDirectionResult::NotFound
    }
}

/// #warp-03 — render a collapsed pane as a thin in-place rail: a non-flexible
/// band with a single expand chevron, clickable to restore the pane. The
/// chevron points the way the pane will grow back, derived from the rail's edge
/// (its position in the parent branch) so it is correct on every side.
fn create_rail(
    axis: SplitDirection,
    is_leading_edge: bool,
    pane_ids: Vec<PaneId>,
    theme: &WarpTheme,
) -> Box<dyn Element> {
    let chevron_icon = match (axis, is_leading_edge) {
        // Vertical branch → row rail: leading (top) expands down, trailing (bottom) expands up.
        (SplitDirection::Vertical, true) => Icon::ChevronDown,
        (SplitDirection::Vertical, false) => Icon::ChevronUp,
        // Horizontal branch → column rail: leading (left) expands right, trailing (right) expands left.
        (SplitDirection::Horizontal, true) => Icon::ChevronRight,
        (SplitDirection::Horizontal, false) => Icon::ChevronLeft,
    };

    let chevron = ConstrainedBox::new(
        chevron_icon
            .to_warpui_icon(theme.active_ui_text_color())
            .finish(),
    )
    .with_width(RAIL_CHEVRON_SIZE)
    .with_height(RAIL_CHEVRON_SIZE)
    .finish();

    // Center the chevron in a band that fills the cross axis and is thin along
    // the rail's edge: a row rail (vertical branch) is full-width / thin-height;
    // a column rail (horizontal branch) is thin-width / full-height.
    // A railed pane reads as a distinct, clickable strip: a background band
    // (so it stands out from the active pane) with the chevron centered on top.
    // Stack[ filled bg, centered chevron ] — same bg+overlay pattern as the
    // color dot; the bg fills the band via the divider's Rect+background pattern.
    let rail_background = theme.split_pane_border_color();
    let band = match axis {
        SplitDirection::Vertical => Stack::new()
            .with_child(
                ConstrainedBox::new(Rect::new().with_background(rail_background).finish())
                    .with_height(RAIL_THICKNESS)
                    .finish(),
            )
            .with_child(
                ConstrainedBox::new(
                    Flex::row()
                        .with_main_axis_size(MainAxisSize::Max)
                        .with_main_axis_alignment(MainAxisAlignment::Center)
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(chevron)
                        .finish(),
                )
                .with_height(RAIL_THICKNESS)
                .finish(),
            )
            .finish(),
        SplitDirection::Horizontal => Stack::new()
            .with_child(
                ConstrainedBox::new(Rect::new().with_background(rail_background).finish())
                    .with_width(RAIL_THICKNESS)
                    .finish(),
            )
            .with_child(
                ConstrainedBox::new(
                    Flex::column()
                        .with_main_axis_size(MainAxisSize::Max)
                        .with_main_axis_alignment(MainAxisAlignment::Center)
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(chevron)
                        .finish(),
                )
                .with_width(RAIL_THICKNESS)
                .finish(),
            )
            .finish(),
    };

    EventHandler::new(band)
        .on_left_mouse_down(move |ctx, _, _| {
            ctx.dispatch_typed_action(PaneGroupAction::RestoreCollapsedGroup(pane_ids.clone()));
            DispatchEventResult::StopPropagation
        })
        .finish()
}

/// Create an invisible placeholder element that occupies the same space as the divider
/// and saves its position so the actual divider can be anchored to it.
fn create_divider_placeholder(direction: SplitDirection, position_id: &str) -> Box<dyn Element> {
    let divider_thickness = get_divider_thickness() - 1.0;

    let placeholder = match direction {
        SplitDirection::Horizontal => ConstrainedBox::new(Empty::new().finish())
            .with_width(divider_thickness)
            .finish(),
        SplitDirection::Vertical => ConstrainedBox::new(Empty::new().finish())
            .with_height(divider_thickness)
            .finish(),
    };

    SavePosition::new(placeholder, position_id).finish()
}

fn divider_mouse_down_action(
    mouse_state: &MouseStateHandle,
    border_id: EntityId,
    direction: SplitDirection,
    position: Vector2F,
) -> PaneGroupAction {
    if mouse_state.lock().unwrap().click_count() == Some(2) {
        PaneGroupAction::ResetPaneSizes(border_id)
    } else {
        PaneGroupAction::StartResizing(DraggedBorder {
            border_id,
            direction,
            previous_mouse_location: position,
        })
    }
}

fn create_divider(
    direction: SplitDirection,
    item: &Divider,
    theme: &WarpTheme,
) -> Box<dyn Element> {
    let divider = ConstrainedBox::new(
        Rect::new()
            .with_background(theme.split_pane_border_color())
            .finish(),
    );

    let cursor_shape = match direction {
        SplitDirection::Horizontal => Cursor::ResizeLeftRight,
        SplitDirection::Vertical => Cursor::ResizeUpDown,
    };

    let border_id = item.id;
    let mouse_state = item.mouse_state.clone();

    Hoverable::new(item.mouse_state.clone(), |_| match direction {
        SplitDirection::Horizontal => divider.with_width(get_divider_thickness()).finish(),
        SplitDirection::Vertical => divider.with_height(get_divider_thickness()).finish(),
    })
    .on_mouse_down(move |ctx, _, position| {
        ctx.dispatch_typed_action(divider_mouse_down_action(
            &mouse_state,
            border_id,
            direction,
            position,
        ));
    })
    .with_cursor(cursor_shape)
    .with_propagate_drag()
    .finish()
}

fn create_minimalist_divider(
    direction: SplitDirection,
    item: &Divider,
    theme: &WarpTheme,
) -> Box<dyn Element> {
    let divider = ConstrainedBox::new(
        Rect::new()
            .with_background(theme.split_pane_border_color())
            .finish(),
    );

    let cursor_shape = match direction {
        SplitDirection::Horizontal => Cursor::ResizeLeftRight,
        SplitDirection::Vertical => Cursor::ResizeUpDown,
    };

    let border_id = item.id;
    let mouse_state = item.mouse_state.clone();
    let hoverable = Hoverable::new(item.mouse_state.clone(), |_| match direction {
        SplitDirection::Horizontal => {
            Container::new(divider.with_width(get_divider_thickness()).finish())
                .with_padding_left(DIVIDER_RESIZE_PADDING)
                .with_padding_right(DIVIDER_RESIZE_PADDING)
                .finish()
        }
        SplitDirection::Vertical => {
            Container::new(divider.with_height(get_divider_thickness()).finish())
                .with_padding_top(DIVIDER_RESIZE_PADDING)
                .with_padding_bottom(DIVIDER_RESIZE_PADDING)
                .finish()
        }
    })
    .on_mouse_down(move |ctx, _, position| {
        ctx.dispatch_typed_action(divider_mouse_down_action(
            &mouse_state,
            border_id,
            direction,
            position,
        ));
    })
    .with_cursor(cursor_shape)
    .with_propagate_drag();

    let mut stack = Stack::new().with_constrain_absolute_children();
    match direction {
        SplitDirection::Horizontal => stack.add_positioned_child(
            hoverable.finish(),
            OffsetPositioning::offset_from_parent(
                Vector2F::new(0., 0.),
                ParentOffsetBounds::Unbounded,
                ParentAnchor::TopMiddle,
                ChildAnchor::TopMiddle,
            ),
        ),
        SplitDirection::Vertical => stack.add_positioned_child(
            hoverable.finish(),
            OffsetPositioning::offset_from_parent(
                Vector2F::new(0., -DIVIDER_RESIZE_PADDING),
                ParentOffsetBounds::Unbounded,
                ParentAnchor::TopLeft,
                ChildAnchor::TopLeft,
            ),
        ),
    };
    stack.finish()
}

impl fmt::Debug for PaneData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Root({:?})", self.root)
    }
}

impl fmt::Debug for PaneNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneNode::Leaf(pane) => write!(f, "Leaf({pane:?})"),
            PaneNode::Branch(branch) => write!(f, "Branch {branch:?}"),
        }
    }
}

impl fmt::Debug for PaneBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.axis {
            SplitDirection::Horizontal => write!(f, "Horizontal({:?})", self.nodes),
            SplitDirection::Vertical => write!(f, "Vertical({:?})", self.nodes),
        }
    }
}

// When pane group is split horizontally, new panes are added from left to right.
// When pane group is split vertically, new panes are added from top to bottom.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

impl From<app_state::SplitDirection> for SplitDirection {
    fn from(direction: app_state::SplitDirection) -> Self {
        match direction {
            app_state::SplitDirection::Horizontal => SplitDirection::Horizontal,
            app_state::SplitDirection::Vertical => SplitDirection::Vertical,
        }
    }
}

impl From<SplitDirection> for app_state::SplitDirection {
    fn from(direction: SplitDirection) -> Self {
        match direction {
            SplitDirection::Horizontal => app_state::SplitDirection::Horizontal,
            SplitDirection::Vertical => app_state::SplitDirection::Vertical,
        }
    }
}

impl From<crate::launch_configs::launch_config::SplitDirection> for SplitDirection {
    fn from(direction: crate::launch_configs::launch_config::SplitDirection) -> Self {
        match direction {
            crate::launch_configs::launch_config::SplitDirection::Horizontal => {
                SplitDirection::Horizontal
            }
            crate::launch_configs::launch_config::SplitDirection::Vertical => {
                SplitDirection::Vertical
            }
        }
    }
}
