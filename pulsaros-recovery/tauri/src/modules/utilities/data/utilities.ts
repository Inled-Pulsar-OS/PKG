export type UtilityAction = "backup" | "reinstall" | "internet" | "disk";

export interface UtilityOption {
  id: UtilityAction;
  title: string;
  desc: string;
  icon: string;
}

export const UTILITIES: UtilityOption[] = [
  {
    id: "backup",
    title: "Restore from Time Machine",
    desc: "If you have a backup of your system that you want to restore.",
    icon: "/icons/timemachine.png",
  },
  {
    id: "reinstall",
    title: "Reinstall Pulsar OS",
    desc: "Install a fresh copy of Pulsar OS while keeping your home files intact.",
    icon: "/icons/reinstall.png",
  },
  {
    id: "internet",
    title: "Pulsar Internet Recovery",
    desc: "Download latest recovery image from GitHub Releases and restore core system.",
    icon: "/icons/internet.png",
  },
  {
    id: "disk",
    title: "Disk Utility",
    desc: "Repair, inspect, or manage disk partitions with GParted.",
    icon: "/icons/diskutility.png",
  },
];
