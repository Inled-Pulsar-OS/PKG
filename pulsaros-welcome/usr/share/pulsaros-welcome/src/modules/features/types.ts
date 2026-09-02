export interface ProviderLog {
    name: string;
    src: string;
}

export interface FeatureSlide {
    id: string;
    title: string;
    subtitle: string;
    video?: string;
    providers?: ProviderLog[];
}