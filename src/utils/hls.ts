import { invoke } from '@tauri-apps/api/tauri';

interface IApiResponse {
  message: string;
  success: boolean;
  playlist_url: string;
  duration: number;
};

interface IResult {
  url: string;
  duration: number;
};

export const getHlsUrl = async (inputPath: string) => new Promise<IResult>(async (resolve, reject) => {
  const response = await invoke<IApiResponse>('generate_hls', { inputPath }).catch(e => {
    throw new Error(e);
  });
  const { success, playlist_url: url, message, duration } = response;
  if(success && url) {
    return resolve({
      url,
      duration,
    });
  } else reject(message);
});