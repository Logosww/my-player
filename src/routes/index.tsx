import { component$, $ } from '@builder.io/qwik';
import { HiPlayCircleSolid } from '@qwikest/icons/heroicons';
import { open } from '@tauri-apps/api/dialog';
import { emit } from '@tauri-apps/api/event';

export default component$(() => {
  const selectVideo = $(async () => {
    // Open a selection dialog for image files
    const selectedFilePath = await open({
      multiple: false,
      filters: [{
        name: 'Video',
        extensions: ['mp4', 'avi', 'mkv', '3gp']
      }]
    });
    if (selectedFilePath) {
      emit('open-video', selectedFilePath);
    }
  });
  return (
    <>
     <button 
        class="absolute select-none left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-50 text-slate-200 flex items-center px-6 py-3 border-slate-500 border rounded-3xl bg-slate-900 hover:bg-slate-500 hover:border-slate-300 transition-colors"
        onClick$={selectVideo}  
      >
        <HiPlayCircleSolid class="inline mr-2 text-[3em]" />
        打开 / 播放
     </button>
    </>
  );
});

