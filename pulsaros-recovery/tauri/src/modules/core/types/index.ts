export interface BtrfsTarget {
  disk_path: string;
  part_path: string;
  label: string;
  uuid: string;
  size: string;
}

export type SystemMode = "recovery" | "installer" | "ootb";

export type RecoveryMode = "local" | "internet";

export type Screen =
  | "utilities"
  | "reinstall_choice"
  | "target_select"
  | "progress"
  | "complete"
  | "error";

export type InstallerScreen =
  | "welcome"
  | "disk_select"
  | "progress"
  | "complete"
  | "error";

export type OotbScreen =
  | "country"
  | "language"
  | "keymap"
  | "timezone"
  | "account"
  | "theme"
  | "progress"
  | "finished";
