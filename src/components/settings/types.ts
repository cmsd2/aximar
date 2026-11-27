import type { AppConfig } from "../../lib/config-client";

/** What every Settings section gets: the config, and a way to change it */
export interface SectionProps {
  config: AppConfig;
  /** Change settings: applied and saved at once */
  update: (updates: Partial<AppConfig>) => void;
}
