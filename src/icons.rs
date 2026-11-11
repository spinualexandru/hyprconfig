//! Icons. The line icons are Lucide's (https://lucide.dev), ISC License,
//! Copyright (c) Lucide Icons and Contributors; the bodies are copied unchanged from
//! lucide-react 1.70.0. The Hyprland mark is Simple Icons' (CC0).

use rune::Icon;

const fn lucide(name: &'static str, body: &'static str) -> Icon {
    Icon {
        name,
        view: 24.0,
        body,
    }
}

pub static HYPRLAND: Icon = Icon {
    name: "hyprland",
    view: 24.0,
    body: r#"<path fill="currentColor" d="M13.0998.001c.071.0753.1162.1116.1476.1576.5693.8424 1.13 1.691 1.7078 2.5277.3326.4816.6866.9486 1.0416 1.4141.3433.451.7058.8869 1.0494 1.3378.3975.5215.8003 1.0402 1.1696 1.5813.473.6933.9496 1.388 1.3588 2.1188.348.6213.626 1.285.8919 1.9477.1882.4691.3144.9646.4491 1.4537.0624.2267.0799.4659.1163.6994.0449.2858.1211.5717.1258.8583.0125.7465.0773 1.4954-.0528 2.2386-.1433.8195-.3832 1.6041-.74 2.3609-.3607.7646-.7978 1.4708-1.3328 2.1241-.4848.5921-1.054 1.0812-1.6697 1.5296-.452.329-.93.6085-1.4387.8306-.4577.1996-.9293.3597-1.4148.4951-1.0961.3052-2.2076.3722-3.3269.2934-.6747-.0474-1.3574-.1301-2-.3729-.5284-.1996-1.0606-.3967-1.5657-.6462-.3988-.1968-.781-.4435-1.131-.7183-.4591-.3604-.9097-.7418-1.3072-1.167-.356-.3808-.6644-.8153-.941-1.2587-.2916-.468-.5486-.9629-.7693-1.4687-.2385-.546-.3925-1.121-.518-1.71-.1953-.9164-.1322-1.8336-.0923-2.748.0229-.525.1615-1.0504.2945-1.5634.14-.541.3094-1.0786.5147-1.5983.2057-.5205.4445-1.0324.718-1.5204.3432-.6124.7218-1.2066 1.1121-1.7905.43-.5304.7372-1.0423 1.1218-1.5513.3098-.41.6477-.7988.9614-1.206.3172-.4116.6224-.8326.9293-1.2522.3247-.4441.652-.8865.9656-1.3385.3979-.5728.781-1.156 1.1753-1.7314.0563-.0823.1383-.1475.2082-.2206.0139.0078.0278.0157.042.0235.003.0631.0082.1262.0082.1893.0004.9386.0053 1.8771-.006 2.8157-.0019.1376-.0578.2905-.1327.4085-.303.4795-.6127.9553-.9403 1.4184-.3105.4388-.6459.8601-.9703 1.289-.2673.3532-.5336.7068-.8027 1.0587-.1868.2441-.3829.4812-.5654.7286-.2552.3465-.5115.6933-.7496 1.0515-.2873.432-.57.8684-.828 1.3179-.1847.3215-.3302.6662-.4845 1.0041-.0856.1875-.1644.3793-.2267.5754-.1105.3479-.199.7026-.3094 1.0505-.3109.9774-.245 1.983-.1754 2.9764.0353.5044.2178 1.0035.3707 1.4933.2142.6858.5817 1.294.9963 1.88.283.4003.6042.762.9675 1.0822.299.263.6238.5051.9649.71.3882.2332.7924.4553 1.2163.6085.4523.1637.9303.2849 1.4073.3451.5678.0717 1.1464.0563 1.72.0834.5325.025 1.049-.0816 1.5659-.1832.6562-.129 1.2558-.412 1.8386-.7222.6969-.3707 1.2826-.8958 1.7955-1.4875.5244-.605.9646-1.285 1.2448-2.0319.2955-.7878.5418-1.603.5044-2.4688-.0203-.468.0053-.939-.0378-1.4041-.0317-.3426-.129-.6813-.2196-1.016-.1062-.3932-.215-.7878-.3622-1.1667-.1465-.3775-.3336-.7396-.5175-1.1014-.163-.3205-.3323-.6388-.5226-.9432a16.5743 16.5743 0 0 0-.7372-1.0848c-.3939-.5311-.8106-1.0451-1.2088-1.5727-.4662-.6177-.9314-1.2362-1.382-1.8654-.3889-.5425-.7596-1.0982-1.1303-1.6536-.0595-.0891-.107-.2064-.108-.3108-.0082-1.0156-.0053-2.0312-.0046-3.0467 0-.0339.0086-.0681.0206-.1583z"/>"#,
};

pub static SETTINGS: Icon = lucide(
    "lucide-settings",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"/><circle cx="12" cy="12" r="3"/></g>"#,
);
pub static PALETTE: Icon = lucide(
    "lucide-palette",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z"/><circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/><circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/><circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/><circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/></g>"#,
);
pub static WIFI: Icon = lucide(
    "lucide-wifi",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h.01"/><path d="M2 8.82a15 15 0 0 1 20 0"/><path d="M5 12.859a10 10 0 0 1 14 0"/><path d="M8.5 16.429a5 5 0 0 1 7 0"/></g>"#,
);
pub static VOLUME_2: Icon = lucide(
    "lucide-volume-2",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4.702a.705.705 0 0 0-1.203-.498L6.413 7.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298z"/><path d="M16 9a5 5 0 0 1 0 6"/><path d="M19.364 18.364a9 9 0 0 0 0-12.728"/></g>"#,
);
pub static MONITOR: Icon = lucide(
    "lucide-monitor",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/></g>"#,
);
pub static KEYBOARD: Icon = lucide(
    "lucide-keyboard",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 8h.01"/><path d="M12 12h.01"/><path d="M14 8h.01"/><path d="M16 12h.01"/><path d="M18 8h.01"/><path d="M6 8h.01"/><path d="M7 16h10"/><path d="M8 12h.01"/><rect width="20" height="16" x="2" y="4" rx="2"/></g>"#,
);
pub static DOLLAR_SIGN: Icon = lucide(
    "lucide-dollar-sign",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v18"/><path d="M17 5H9.5a1 1 0 000 7h5a1 1 0 010 7H6"/></g>"#,
);
pub static TERMINAL: Icon = lucide(
    "lucide-terminal",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19h8"/><path d="m4 17 6-6-6-6"/></g>"#,
);
pub static APP_WINDOW: Icon = lucide(
    "lucide-app-window",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M10 4v4"/><path d="M2 8h20"/><path d="M6 4v4"/></g>"#,
);
pub static LAYERS: Icon = lucide(
    "lucide-layers",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z"/><path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12"/><path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17"/></g>"#,
);
pub static INFO: Icon = lucide(
    "lucide-info",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></g>"#,
);
pub static SUN: Icon = lucide(
    "lucide-sun",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></g>"#,
);
pub static MOON: Icon = lucide(
    "lucide-moon",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20.985 12.486a9 9 0 1 1-9.473-9.472c.405-.022.617.46.402.803a6 6 0 0 0 8.268 8.268c.344-.215.825-.004.803.401"/></g>"#,
);
pub static REFRESH_CW: Icon = lucide(
    "lucide-refresh-cw",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/></g>"#,
);
pub static PLUS: Icon = lucide(
    "lucide-plus",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></g>"#,
);
pub static TRASH_2: Icon = lucide(
    "lucide-trash-2",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10 11v6"/><path d="M14 11v6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></g>"#,
);
pub static PENCIL: Icon = lucide(
    "lucide-pencil",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z"/><path d="m15 5 4 4"/></g>"#,
);
pub static CHECK: Icon = lucide(
    "lucide-check",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></g>"#,
);
pub static X: Icon = lucide(
    "lucide-x",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></g>"#,
);
pub static CHEVRON_DOWN: Icon = lucide(
    "lucide-chevron-down",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></g>"#,
);
pub static CHEVRON_RIGHT: Icon = lucide(
    "lucide-chevron-right",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></g>"#,
);
pub static GLOBE: Icon = lucide(
    "lucide-globe",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/></g>"#,
);
pub static UPLOAD: Icon = lucide(
    "lucide-upload",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12"/><path d="m17 8-5-5-5 5"/><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/></g>"#,
);
pub static IMAGE: Icon = lucide(
    "lucide-image",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></g>"#,
);
pub static FILE_CODE: Icon = lucide(
    "lucide-file-code",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z"/><path d="M14 2v5a1 1 0 0 0 1 1h5"/><path d="M10 12.5 8 15l2 2.5"/><path d="m14 12.5 2 2.5-2 2.5"/></g>"#,
);
pub static MIC: Icon = lucide(
    "lucide-mic",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19v3"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><rect x="9" y="2" width="6" height="13" rx="3"/></g>"#,
);
pub static MIC_OFF: Icon = lucide(
    "lucide-mic-off",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 19v3"/><path d="M15 9.34V5a3 3 0 0 0-5.68-1.33"/><path d="M16.95 16.95A7 7 0 0 1 5 12v-2"/><path d="M18.89 13.23A7 7 0 0 0 19 12v-2"/><path d="m2 2 20 20"/><path d="M9 9v3a3 3 0 0 0 5.12 2.12"/></g>"#,
);
pub static VOLUME_OFF: Icon = lucide(
    "lucide-volume-off",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 9a5 5 0 0 1 .95 2.293"/><path d="M19.364 5.636a9 9 0 0 1 1.889 9.96"/><path d="m2 2 20 20"/><path d="m7 7-.587.587A1.4 1.4 0 0 1 5.416 8H3a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2.416a1.4 1.4 0 0 1 .997.413l3.383 3.384A.705.705 0 0 0 11 19.298V11"/><path d="M9.828 4.172A.686.686 0 0 1 11 4.657v.686"/></g>"#,
);
pub static SPEAKER: Icon = lucide(
    "lucide-speaker",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="16" height="20" x="4" y="2" rx="2"/><path d="M12 6h.01"/><circle cx="12" cy="14" r="4"/><path d="M12 14h.01"/></g>"#,
);
pub static MUSIC: Icon = lucide(
    "lucide-music",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V3a1 1 0 0 0-1.28-.96l-10 2.917a1 1 0 0 0-.72.96V19"/><circle cx="18" cy="16" r="3"/><circle cx="6" cy="19" r="3"/></g>"#,
);
pub static CABLE: Icon = lucide(
    "lucide-cable",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 19a1 1 0 0 1-1-1v-2a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2a1 1 0 0 1-1 1z"/><path d="M17 21v-2"/><path d="M19 14V6.5a1 1 0 0 0-7 0v11a1 1 0 0 1-7 0V10"/><path d="M21 21v-2"/><path d="M3 5V3"/><path d="M4 10a2 2 0 0 1-2-2V6a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2a2 2 0 0 1-2 2z"/><path d="M7 5V3"/></g>"#,
);
pub static NETWORK: Icon = lucide(
    "lucide-network",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="16" y="16" width="6" height="6" rx="1"/><rect x="2" y="16" width="6" height="6" rx="1"/><rect x="9" y="2" width="6" height="6" rx="1"/><path d="M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3"/><path d="M12 12V8"/></g>"#,
);
pub static SHIELD_CHECK: Icon = lucide(
    "lucide-shield-check",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/></g>"#,
);
pub static SMARTPHONE: Icon = lucide(
    "lucide-smartphone",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="14" height="20" x="5" y="2" rx="2" ry="2"/><path d="M12 18h.01"/></g>"#,
);
pub static ARROW_LEFT: Icon = lucide(
    "lucide-arrow-left",
    r#"<g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></g>"#,
);
