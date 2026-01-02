export interface AudioDevice {
  id: number;
  name: string;
  description: string;
  device_type: "sink" | "source";
  volume: number;
  muted: boolean;
  is_default: boolean;
}

export interface AudioStream {
  id: number;
  app_name: string;
  media_name?: string;
  volume: number;
  muted: boolean;
}

export interface AudioState {
  sinks: AudioDevice[];
  sources: AudioDevice[];
  streams: AudioStream[];
}
