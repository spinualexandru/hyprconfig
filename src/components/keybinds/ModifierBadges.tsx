import { Badge } from "@/components/ui/badge";
import { getModifierVariant } from "@/utils/keybinds";

interface ModifierBadgesProps {
  modifiers: string[];
}

export function ModifierBadges({ modifiers }: ModifierBadgesProps) {
  if (modifiers.length === 0) {
    return <span className="text-muted-foreground text-sm">None</span>;
  }

  return (
    <div className="flex gap-1 flex-wrap">
      {modifiers.map((mod, i) => (
        <Badge key={i} variant={getModifierVariant(mod)}>
          {mod}
        </Badge>
      ))}
    </div>
  );
}
