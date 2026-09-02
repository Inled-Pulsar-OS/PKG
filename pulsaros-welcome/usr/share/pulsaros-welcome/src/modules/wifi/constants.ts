import type { ConnectForm } from "./types";

export const INITIAL_FORM: ConnectForm = {
    selected: null,
    password: "",
    showPassword: false,
    connecting: false,
    success: false,
    error: null,
};
