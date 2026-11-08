export interface FeatureItem {
  title: string;
  description: string;
  video: string;
  poster?: string;
}

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
