import { $, component$, useVisibleTask$, Slot, useSignal, useStore, noSerialize } from '@builder.io/qwik';
import { isServer } from '@builder.io/qwik/build';
import { listen } from '@tauri-apps/api/event';
import { appWindow, LogicalSize } from '@tauri-apps/api/window';
import { getHlsUrl, getSubtileUrl } from '~/utils';
import { LuX } from '@qwikest/icons/lucide';
import TitleBar from '~/components/layouts/title-bar';
import SpinnerIcon from '~/components/icons/spinner';
import SubtitleIcon from '~/components/icons/subtitle';
import videojs from 'video.js';
import 'video.js/dist/video-js.css';

import type { NoSerialize, QRL } from '@builder.io/qwik';
import type { EventCallback } from '@tauri-apps/api/event';
import type Player from 'video.js/dist/types/player';
import type HTMLTrackElement from 'video.js/dist/types/tracks/html-track-element';

export default component$(() => {
  const videoPath = useSignal('');
  const isLoading = useSignal(false);
  const isShowControl = useSignal(false);
  const isLoadingSubtitle = useSignal(false);
  const duration = useSignal<number>();
  const store = useStore<{
    player: NoSerialize<Player>; 
    currentVideoPath: string;
    subtitleTrackEl: NoSerialize<HTMLTrackElement>;
  }>({
    player: void 0, 
    currentVideoPath: '', 
    subtitleTrackEl: void 0,
  });

  const handleOpenVideo: QRL<EventCallback<string>> = $(async ({ payload }) => {
    if(!store.player || isServer) return;

    videoPath.value = store.currentVideoPath = payload;
    isLoading.value = true;
    const { url: hlsUrl, duration: _duration } = await getHlsUrl(videoPath.value);
    duration.value = _duration;
    store.player!.src(hlsUrl);
    store.player!.load();
    store.player!.play();
  });

  const setSize = $(() => {
    if(!store.player || isServer) return;
    const _size = new LogicalSize(
      store.player.videoWidth(),
      store.player.videoHeight() + 40,
    );
    appWindow.setSize(_size).then(() => {
      duration.value && store.player!.duration(duration.value);
      isLoading.value = false;
    });
  });

  const handleToggleFullScreen = $(() => {
    if(!store.player || isServer) return;
    
    const isFullScreen = store.player!.isFullscreen()!;
    appWindow.setFullscreen(isFullScreen);
  });

  const handleToggleSubtitle = $(async () => {
    if(isLoadingSubtitle.value || !store.currentVideoPath) return;

    if(store.subtitleTrackEl) {
      store.player?.removeRemoteTextTrack(store.subtitleTrackEl);
      return store.subtitleTrackEl = void 0;
    }

    isLoadingSubtitle.value = true;
    const subtitleUrl = await getSubtileUrl(store.currentVideoPath);
    const trackEl = store.player?.addRemoteTextTrack({ label: '字幕', src: subtitleUrl }, false);
    trackEl?.addEventListener('load', () => {
      store.subtitleTrackEl = noSerialize(trackEl);
      let textTrack;
      // @ts-ignore
      const tracks = store.player!.textTracks() as any[];
      for(let i = 0; i < tracks.length; i++) {
        if(tracks[i].label === '字幕') {
          textTrack = tracks[i];
          break;
        }
      }
      textTrack.mode = 'showing';
      isLoadingSubtitle.value = false;
    })
  });

  const handleQuitPlay = $(() => {
    if(!store.player || isServer) return;

    store.player.currentTime(0);
    store.player.pause();
    videoPath.value = '';
    isLoadingSubtitle.value = false;
    isShowControl.value = false;
    duration.value = void 0;
    isLoading.value = false;
  });
  
  useVisibleTask$(async ({ cleanup }) => {
    const unsubscribeOpenVideo = await listen('open-video', handleOpenVideo);
    const unsubscribeFileDrop = await listen('file-drop-event', handleOpenVideo);
    store.player = store.player || noSerialize(videojs('my-player', {
      preload: false,
      controls: true,
      fill: true,
      enableSmoothSeeking: true,
    }, () => {
      store.player!.on('loadedmetadata', () => setSize());
      store.player!.on('fullscreenchange', () => handleToggleFullScreen());

      const playerEl = document.getElementById('my-player')!;
      const observer = new MutationObserver(list => {
        if(!videoPath.value) return;

        for(let mutation of list) {
          mutation.type === 'attributes' 
          && mutation.attributeName === 'class'
          && (isShowControl.value = !playerEl?.classList.contains('vjs-user-inactive'))
        }
      });
      observer.observe(playerEl, { attributes: true, attributeFilter: ['class'] });

      store.player!.on('dispose', () => observer.disconnect());

      cleanup(() => {
        store.player?.dispose();
        unsubscribeOpenVideo();
        unsubscribeFileDrop();
      });
    }));
  }, { strategy: 'document-ready' });

  return (
    <>
      <TitleBar />
      <div class="fixed bg-slate-800 pt-10 w-full h-full">
        <Slot />
        <div class="relative w-full h-full z-50" hidden={!videoPath.value} data-vjs-player>
          <div id="player-container" class="w-full h-full fade-in" hidden={isLoading.value}>
            <video-js id="my-player" />
          </div>
          <div id="loading-container" class="absolute w-full h-full bg-slate-800 flex justify-center items-center fade-in" hidden={!isLoading.value}>
            <SpinnerIcon class="text-[4em] text-slate-200" />
          </div>
        </div>
        <div
          class="fixed top-1/2 text-white text-[1.8em] p-[6px] rounded-lg right-[16px] -translate-y-1/2 backdrop-blur-md transition-colors cursor-pointer z-[100] fade-in"
          style={{ backgroundColor: 'rgba(0, 0, 0, .3)' }}
          hidden={!isShowControl.value}
          onClick$={handleQuitPlay}
        >
          <LuX />
        </div>
        <div
          class="fixed top-1/2 text-white text-[1.8em] p-[6px] rounded-lg left-[16px] -translate-y-1/2 backdrop-blur-md transition-colors cursor-pointer z-[100] fade-in"
          style={{ backgroundColor: store.subtitleTrackEl ? 'rgba(255, 255, 255, .7)' : 'rgba(0, 0, 0, .3)'}}
          hidden={!isShowControl.value}
          onClick$={handleToggleSubtitle}
        >
          <SubtitleIcon class={{ invisible: isLoadingSubtitle.value, 'text-black': !!store.subtitleTrackEl, 'transition-colors': true }} />
          <SpinnerIcon 
            class={{
              'top-[6px]': true,
              'left-[6px]': true, 
              'bottom-[6px]': true,
              'right-[6px]': true,
              absolute: true,
              invisible: !isLoadingSubtitle.value,
              visible: isLoadingSubtitle.value,
            }}
          />
        </div>
      </div>
    </>
  );
});
