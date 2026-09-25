import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { ConfigInfo } from "@/types/config";

export function useConfigInfo() {
	const [configInfo, setConfigInfo] = useState<ConfigInfo | null>(null);

	useEffect(() => {
		invoke<ConfigInfo>("get_config_info")
			.then(setConfigInfo)
			.catch((err) => console.error("Failed to load config info:", err));
	}, []);

	return configInfo;
}
