export type SystemMode = "welcome";

export type WelcomeScreen =
  | "hello"
  | "features"
  | "compatibility"
  | "settings"
  | "sayri"
  | "recovery"
  | "wifi"
  | "done";

export interface Resolution {
  width: number;
  height: number;
  active: boolean;
}


export type DataState = {
  isLive: boolean;
  isArch: boolean;
  ootbPending: boolean;
  resolutions: Resolution[];
  effectsState: boolean;
};