export interface BtrfsTarget {
  disk_path: string;
  part_path: string;
  label: string;
  uuid: string;
  size: string;
}

export type RecoveryMode = "local" | "internet";

export type Screen =
  | "utilities"
  | "target_select"
  | "progress"
  | "complete"
  | "error";
