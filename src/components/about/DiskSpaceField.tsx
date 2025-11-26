import {Progress} from "@/components/ui/progress";

interface DiskSpaceFieldProps {
    label: string;
    used: string;
    total: string;
    className?: string;
}

function parseDiskSpace(value: string): number {
    // Parse values like "45.2 G", "1.5 T", "500 M" (disk)
    // or "8.5 GB", "16 GB" (RAM)
    const matchShort = value.match(/^([\d.]+)\s*([KMGT])$/i);
    const matchLong = value.match(/^([\d.]+)\s*([KMGT]B)$/i);

    const match = matchShort || matchLong;
    if (!match) return 0;

    const num = Number.parseFloat(match[1]);
    const unit = match[2].toUpperCase().replace('B', ''); // Remove 'B' to normalize

    // Convert to GB for consistent calculation
    switch (unit) {
        case "T":
            return num * 1024;
        case "G":
            return num;
        case "M":
            return num / 1024;
        case "K":
            return num / (1024 * 1024);
        default:
            return num;
    }
}

export function DiskSpaceField({
                                   label,
                                   used,
                                   total,
                                   className = "",
                               }: DiskSpaceFieldProps) {

    const usedGB = parseDiskSpace(used);
    const totalGB = parseDiskSpace(total);
    const percentage = totalGB > 0 ? (usedGB / totalGB) * 100 : 0;

    return (
        <div className={className}>
            <p className="text-sm font-medium text-foreground mb-2">{label}</p>
            <p className="text-sm text-muted-foreground mt-2">
                {used} / {total}
            </p>
            <Progress value={percentage} className="h-3 max-w-72"/>

        </div>
    );
}
