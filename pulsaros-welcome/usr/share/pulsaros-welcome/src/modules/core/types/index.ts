export type SystemMode = "welcome";

export type WelcomeScreen =
  | "hello"
  | "intro"
  | "resolution"
  | "wifi"
  | "bluetooth"
  | "software"
  | "effects"
  | "gpu"
  | "feedback"
  | "done";

export interface Resolution {
  width: number;
  height: number;
  active: boolean;
}
