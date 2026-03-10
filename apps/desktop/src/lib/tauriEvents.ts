import { emit, listen } from "@tauri-apps/api/event";

export type UnlistenFn = () => void;

export async function listenEvent<T>(
  eventName: string,
  handler: (payload: T) => void
): Promise<UnlistenFn> {
  return listen<T>(eventName, (event) => {
    handler(event.payload);
  });
}

export function emitEvent<T>(eventName: string, payload: T): Promise<void> {
  return emit(eventName, payload);
}
