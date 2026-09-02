export interface WifiNetwork {
    ssid: string;
    signal: number;
    security: string;
}

export interface ConnectForm {
    selected: string | null;
    password: string;
    showPassword: boolean;
    connecting: boolean;
    success: boolean;
    error: string | null;
}
