/* eslint-disable @typescript-eslint/no-explicit-any */

/**
 * This is a fork of the vue-click-away plugin:
 * https://github.com/VinceG/vue-click-away
 *
 * The main modification is that it detects `mousedown` instead of `click`.
 */

const CLICK_EVENT_TYPE = "mousedown";

const UNIQUE_ID = "__vue_click_away_mousedown_fork__";

/**
 * Typescript interface for Vue's "binding" argument, see:
 * https://vuejs.org/guide/reusability/custom-directives.html#hook-arguments
 */
interface Binding {
  // The value passed to the directive. For example in `v-my-directive="1 + 1"`, the value would be 2.
  value: any;
  // The previous value, only available in `beforeUpdate` and `updated`. It is available whether or not the value has changed.
  oldValue?: any;
  // The argument passed to the directive, if any. For example in v-my-directive:foo, the arg would be "foo".
  arg: any;
  // An object containing modifiers, if any. For example in v-my-directive.foo.bar, the modifiers object would be { foo: true, bar: true }.
  modifiers: Record<string, boolean | undefined>;
  // The instance of the component where the directive is used.
  instance: any;
  // the directive definition object.
  dir: any;
}

interface MyElement extends Element {
  [UNIQUE_ID]?: (e: MouseEvent) => void;
}

function onMounted(el: MyElement, binding: Binding, vnode: any) {
  onUnmounted(el);

  const vm = vnode.context;
  const callback = binding.value;

  let nextTick = false;
  setTimeout(function () {
    nextTick = true;
  }, 0);

  el[UNIQUE_ID] = (event) => {
    if (
      (!el || !el.contains(event.target as Node)) &&
      callback &&
      nextTick &&
      typeof callback === "function"
    ) {
      return callback.call(vm, event);
    }
  };

  document.addEventListener(CLICK_EVENT_TYPE, el[UNIQUE_ID], false);
}

function onUnmounted(el: MyElement) {
  document.removeEventListener(CLICK_EVENT_TYPE, el[UNIQUE_ID]!, false);
  delete el[UNIQUE_ID];
}

function onUpdated(el: Element, binding: Binding, vnode: any) {
  if (binding.value === binding.oldValue) {
    return;
  }
  onMounted(el, binding, vnode);
}

const plugin = {
  install: (app: any) => {
    app.directive("click-away", directive);
  },
};

const directive = {
  mounted: onMounted,
  updated: onUpdated,
  unmounted: onUnmounted,
};

export default plugin;
