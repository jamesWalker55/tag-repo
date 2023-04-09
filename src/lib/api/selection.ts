import { AppState, state } from "./state";
import { unreachable } from "@/lib/utils";
import { computed } from "vue";

export enum SelectionType {
  RANGE = "range",
  SEPARATE = "separate",
}

interface BaseSelection {
  type: SelectionType;
}

interface RangeSelection extends BaseSelection {
  type: SelectionType.RANGE;
  rootIndex: number;
  extendToIndex: number;
}

interface SeparateSelection extends BaseSelection {
  type: SelectionType.SEPARATE;
  itemIds: number[];
  // for converting to a range selection
  lastToggledId: number;
}

export type Selection = RangeSelection | SeparateSelection;

/** Get the minimum and maximum list index (inclusive) of the range selection */
function getRangeMinMax(rangeSel: RangeSelection): [number, number] {
  let small;
  let large;
  if (rangeSel.rootIndex < rangeSel.extendToIndex) {
    small = rangeSel.rootIndex;
    large = rangeSel.extendToIndex;
  } else {
    small = rangeSel.extendToIndex;
    large = rangeSel.rootIndex;
  }
  return [small, large];
}

/** Convert a range selection to a list of item IDs */
function rangeToItemIds(rangeSel: RangeSelection): number[] {
  const [small, large] = getRangeMinMax(rangeSel);
  return state.itemIds.slice(small, large - small + 1);
}

function createSelectionManager(state: AppState) {
  const selectedItemIds = computed(() => {
    const selection = state.itemIdSelection;
    if (selection === null) {
      return [];
    } else {
      const selectionType = selection.type;
      switch (selectionType) {
        case SelectionType.RANGE:
          return rangeToItemIds(selection);
        case SelectionType.SEPARATE:
          return selection.itemIds;
      }
      unreachable(selectionType);
    }
  });

  function contains(itemId: number): boolean {
    const index = selectedItemIds.value.indexOf(itemId);
    return index !== -1;
  }

  function add(itemId: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      state.itemIdSelection = {
        type: SelectionType.SEPARATE,
        itemIds: [itemId],
        lastToggledId: itemId,
      };
    } else {
      if (contains(itemId)) throw "item id already exists in selection!";

      const itemIds = selectedItemIds.value;
      itemIds.push(itemId);

      state.itemIdSelection = {
        type: SelectionType.SEPARATE,
        itemIds: itemIds,
        lastToggledId: itemId,
      };
    }
  }

  function remove(itemId: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      throw "no active selection!";
    }

    const itemIds = selectedItemIds.value;
    const index = itemIds.indexOf(itemId);
    if (index === -1) throw "item id doesn't exist in selection!";

    itemIds.splice(index, 1);

    state.itemIdSelection = {
      type: SelectionType.SEPARATE,
      itemIds: itemIds,
      lastToggledId: itemId,
    };
  }

  function extendTo(itemId: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      return;
    }

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        const index = state.itemIds.indexOf(itemId);
        if (index === -1) throw "item id not in list!";

        selection.extendToIndex = index;
        return;
      case SelectionType.SEPARATE:
        state.itemIdSelection = {
          type: SelectionType.RANGE,
          rootIndex: selection.lastToggledId,
          extendToIndex: itemId,
        };
        return;
    }
    unreachable(selectionType);
  }

  function clear() {
    state.itemIdSelection = null;
  }

  return {
    selected: selectedItemIds,
    contains: contains,
    add: add,
    remove: remove,
    extendTo: extendTo,
    clear: clear,
  };
}

export const selection = createSelectionManager(state);
