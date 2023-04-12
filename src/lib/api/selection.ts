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
  indexes: number[];
  // for converting to a range selection
  lastToggledIndex: number;
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

/** Convert a range selection to a list of list indexes */
function rangeToArray(rangeSel: RangeSelection): number[] {
  const [small, large] = getRangeMinMax(rangeSel);
  const indexes = [];
  for (let i = small; i <= large; i++) {
    indexes.push(i);
  }
  return indexes;
}

function createSelectionManager(state: AppState) {
  const selectedIndexes = computed(() => {
    const selection = state.itemIdSelection;
    if (selection === null) {
      return [];
    } else {
      const selectionType = selection.type;
      switch (selectionType) {
        case SelectionType.RANGE:
          return rangeToArray(selection);
        case SelectionType.SEPARATE:
          return selection.indexes;
        default:
          unreachable(selectionType);
      }
    }
  });

  const selectedCount = computed(() => selectedIndexes.value.length);

  function itemIdToIndex(itemId: number): number {
    const index = state.itemIds.indexOf(itemId);
    if (index === -1) throw "item id doesn't exist in selection!";

    return index;
  }

  function indexToItemId(index: number): number {
    if (index < 0 || index >= state.itemIds.length)
      throw "item index is out of bounds!";
    return state.itemIds[index];
  }

  function contains(index: number): boolean {
    const selection = state.itemIdSelection;
    if (selection === null) {
      return false;
    } else {
      const selectionType = selection.type;
      switch (selectionType) {
        case SelectionType.RANGE:
          const [small, large] = getRangeMinMax(selection);
          return small <= index && index <= large;
        case SelectionType.SEPARATE:
          return selection.indexes.indexOf(index) !== -1;
        default:
          unreachable(selectionType);
      }
    }
  }

  /** Select a single item, and clear all other selections */
  function isolate(index: number) {
    state.itemIdSelection = {
      type: SelectionType.SEPARATE,
      indexes: [index],
      lastToggledIndex: index,
    };
  }

  function add(index: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      isolate(index);
    } else {
      if (contains(index)) throw "item id already exists in selection!";

      const indexes = selectedIndexes.value.slice(0);
      indexes.push(index);

      state.itemIdSelection = {
        type: SelectionType.SEPARATE,
        indexes: indexes,
        lastToggledIndex: index,
      };
    }
  }

  function addTo(endIndex: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      state.itemIdSelection = {
        type: SelectionType.RANGE,
        rootIndex: 0,
        extendToIndex: endIndex,
      };
    } else {
      const selectionType = selection.type;
      let startIndex: number;
      let smallIndex: number;
      let largeIndex: number;
      let indexes: number[];
      switch (selectionType) {
        case SelectionType.RANGE:
          startIndex = selection.extendToIndex;
          const [small, large] = getRangeMinMax(selection);
          indexes = rangeToArray(selection);
          if (endIndex < small) {
            for (let i = endIndex; i < small; i++) {
              indexes.push(i);
            }
          } else if (endIndex > large) {
            for (let i = large + 1; i <= endIndex; i++) {
              indexes.push(i);
            }
          } else {
            // adding inside the existing selection, just change the format
          }
          state.itemIdSelection = {
            type: SelectionType.SEPARATE,
            indexes: indexes,
            lastToggledIndex: endIndex,
          };
          return;
        case SelectionType.SEPARATE:
          startIndex = selection.lastToggledIndex;
          if (startIndex > endIndex) {
            smallIndex = endIndex;
            largeIndex = startIndex;
          } else {
            smallIndex = startIndex;
            largeIndex = endIndex;
          }
          for (let i = smallIndex; i <= largeIndex; i++) {
            const indexAlreadyExists = selection.indexes.indexOf(i) !== -1;
            if (!indexAlreadyExists) {
              selection.indexes.push(i);
            }
          }
          selection.lastToggledIndex = endIndex;
          return;
        default:
          unreachable(selectionType);
      }
    }
  }

  function remove(index: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      throw "no active selection!";
    }

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        const [small, large] = getRangeMinMax(selection);
        if (!(small <= index && index <= large))
          throw "item id doesn't exist in selection!";
        const indexes = [];
        for (let i = small; i <= large; i++) {
          if (i !== index) {
            indexes.push(i);
          }
        }
        state.itemIdSelection = {
          type: SelectionType.SEPARATE,
          indexes: indexes,
          lastToggledIndex: index,
        };
        return;
      case SelectionType.SEPARATE:
        const indexOfIndex = selection.indexes.indexOf(index);
        if (indexOfIndex === -1) throw "item id doesn't exist in selection!";

        selection.indexes.splice(indexOfIndex, 1);
        selection.lastToggledIndex = index;
        return;
      default:
        unreachable(selectionType);
    }
  }

  function extendTo(index: number) {
    const selection = state.itemIdSelection;
    if (selection === null) {
      state.itemIdSelection = {
        type: SelectionType.RANGE,
        rootIndex: 0,
        extendToIndex: index,
      };
      return;
    }

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        selection.extendToIndex = index;
        return;
      case SelectionType.SEPARATE:
        state.itemIdSelection = {
          type: SelectionType.RANGE,
          rootIndex: selection.lastToggledIndex,
          extendToIndex: index,
        };
        return;
      default:
        unreachable(selectionType);
    }
  }

  function clear() {
    state.itemIdSelection = null;
  }

  /** This is used for keyboard navigation, when you press "down" on the list */
  function isolateDown() {
    // ensure there is at least 1 item in the list
    if (state.itemIds.length === 0) return;

    const maxIndex = state.itemIds.length - 1;

    const selection = state.itemIdSelection;
    if (selection === null) {
      isolate(0);
      return;
    }

    let newIndex: number;

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        newIndex = Math.min(selection.extendToIndex + 1, maxIndex);
        isolate(newIndex);
        return;
      case SelectionType.SEPARATE:
        newIndex = Math.min(selection.lastToggledIndex + 1, maxIndex);
        isolate(newIndex);
        return;
      default:
        unreachable(selectionType);
    }
  }

  /** This is used for keyboard navigation, when you press "up" on the list */
  function isolateUp() {
    // ensure there is at least 1 item in the list
    if (state.itemIds.length === 0) return;

    const maxIndex = state.itemIds.length - 1;

    const selection = state.itemIdSelection;
    if (selection === null) {
      isolate(maxIndex);
      return;
    }

    let newIndex: number;

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        newIndex = Math.max(selection.extendToIndex - 1, 0);
        isolate(newIndex);
        return;
      case SelectionType.SEPARATE:
        newIndex = Math.max(selection.lastToggledIndex - 1, 0);
        isolate(newIndex);
        return;
      default:
        unreachable(selectionType);
    }
  }

  /** This is used for keyboard navigation, when you press "shift+down" on the list */
  function extendDown() {
    // ensure there is at least 1 item in the list
    if (state.itemIds.length === 0) return;

    const maxIndex = state.itemIds.length - 1;

    const selection = state.itemIdSelection;
    if (selection === null) {
      isolate(0);
      return;
    }

    let newIndex: number;

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        newIndex = Math.min(selection.extendToIndex + 1, maxIndex);
        extendTo(newIndex);
        return;
      case SelectionType.SEPARATE:
        newIndex = Math.min(selection.lastToggledIndex + 1, maxIndex);
        extendTo(newIndex);
        return;
      default:
        unreachable(selectionType);
    }
  }

  /** This is used for keyboard navigation, when you press "shift+up" on the list */
  function extendUp() {
    // ensure there is at least 1 item in the list
    if (state.itemIds.length === 0) return;

    const maxIndex = state.itemIds.length - 1;

    const selection = state.itemIdSelection;
    if (selection === null) {
      isolate(maxIndex);
      return;
    }

    let newIndex: number;

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        newIndex = Math.max(selection.extendToIndex - 1, 0);
        extendTo(newIndex);
        return;
      case SelectionType.SEPARATE:
        newIndex = Math.max(selection.lastToggledIndex - 1, 0);
        extendTo(newIndex);
        return;
      default:
        unreachable(selectionType);
    }
  }

  /** This returns the index that is "focused", usually the last-toggled item by the user */
  function focusedIndex(): number | null {
    // ensure there is at least 1 item in the list
    if (state.itemIds.length === 0) return null;

    const selection = state.itemIdSelection;
    if (selection === null) {
      return null;
    }

    const selectionType = selection.type;
    switch (selectionType) {
      case SelectionType.RANGE:
        return selection.extendToIndex;
      case SelectionType.SEPARATE:
        return selection.lastToggledIndex;
      default:
        unreachable(selectionType);
    }
  }

  return {
    selected: selectedIndexes,
    selectedCount: selectedCount,
    itemIdToIndex: itemIdToIndex,
    indexToItemId: indexToItemId,
    contains: contains,
    isolate: isolate,
    add: add,
    addTo: addTo,
    remove: remove,
    extendTo: extendTo,
    clear: clear,
    isolateDown: isolateDown,
    isolateUp: isolateUp,
    extendDown: extendDown,
    extendUp: extendUp,
    focusedIndex: focusedIndex,
  };
}

export const selection = createSelectionManager(state);
