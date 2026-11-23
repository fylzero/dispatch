import { ref } from "vue";

export type ToastKind = "success" | "error";

export const toastMessage = ref("");
export const toastKind = ref<ToastKind>("success");

let timer: number | undefined;

export function showToast(message: string, kind: ToastKind = "success") {
  toastMessage.value = message;
  toastKind.value = kind;
  resumeToast();
}

export function dismissToast() {
  window.clearTimeout(timer);
  toastMessage.value = "";
}

export function pauseToast() {
  window.clearTimeout(timer);
}

export function resumeToast() {
  window.clearTimeout(timer);
  timer = window.setTimeout(dismissToast, toastKind.value === "error" ? 6000 : 3200);
}
