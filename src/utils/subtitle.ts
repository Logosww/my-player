import { invoke } from '@tauri-apps/api/tauri';

interface IApiResponse {
  message: string;
  success: boolean;
  subtitle_url: string;
};

export const getSubtileUrl = async (inputPath: string) => new Promise<string>(async (resolve, reject) => {
  const response = await invoke<IApiResponse>('generate_subtitle', { inputPath }).catch(e => {
    throw new Error(e);
  });
  const { success, subtitle_url: url, message } = response;
  if(success && url) resolve(url);
  else reject(message);
});