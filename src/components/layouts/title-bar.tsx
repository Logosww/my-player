/* eslint-disable qwik/jsx-img */
import { $, component$, useSignal, useVisibleTask$ } from '@builder.io/qwik';
import { appWindow } from '@tauri-apps/api/window';
import { HiStopOutline, HiMinusOutline, HiXMarkOutline } from '@qwikest/icons/heroicons';
import { isServer } from '@builder.io/qwik/build';

export default component$(() => {
  const titleBarRef = useSignal<HTMLDivElement>();

  const handleWindowMinimize = $(() => !isServer && appWindow.minimize());
  const handleToggleWindowMaximize = $(() => !isServer && appWindow.toggleMaximize());
  const handleWindowClose = $(() => !isServer && appWindow.close());

  const doDragging = $(() => {
    if(isServer) return;

    appWindow.startDragging();
  });

  useVisibleTask$(({ cleanup }) => {
    if(titleBarRef.value) {
      titleBarRef.value.addEventListener('mousedown', doDragging);
      titleBarRef.value.addEventListener('dblclick', appWindow.toggleMaximize);
    }

    cleanup(() => {
      titleBarRef.value?.removeEventListener('mousedown', doDragging);
      titleBarRef.value?.removeEventListener('dblclick', appWindow.toggleMaximize);
    });
  });

  return (
    <div class="fixed h-10 w-full inset-x-0 z-10 flex justify-between items-center bg-slate-900 text-white select-none" ref={titleBarRef}>
      <span class="ml-4">My Player</span>
      <div>
        <div class="inline-flex justify-center items-center h-10 w-10 text-lg transition-colors hover:bg-slate-700"
          onClick$={handleWindowMinimize}>
          <HiMinusOutline />
        </div>
        <div class="inline-flex justify-center items-center h-10 w-10 text-lg transition-colors hover:bg-slate-700"
          onClick$={handleToggleWindowMaximize}>
          <HiStopOutline />
        </div>
        <div class="inline-flex justify-center items-center h-10 w-10 text-lg transition-colors hover:bg-rose-500"
          onClick$={handleWindowClose}>
          <HiXMarkOutline />
        </div>
      </div>
    </div>
  )
});