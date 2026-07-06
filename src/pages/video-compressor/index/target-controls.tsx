/*
 * 核心职责：展示视频目标卡片和滑块控件。
 * 业务痛点：编码参数控件重复出现在页面入口会增加阅读成本。
 * 能力边界：只负责受控 UI 展示，不保存业务状态。
 */

import type { ReactNode } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";

export function TargetCard({ checked, onToggle, icon, label }: { checked: boolean; onToggle: () => void; icon: ReactNode; label: ReactNode }) {
  return (
    <Label className="flex cursor-pointer flex-col items-center justify-center rounded-xl border p-4 text-center transition-colors hover:bg-accent hover:text-accent-foreground [&:has([data-state=checked])]:border-primary [&:has([data-state=checked])]:bg-primary/5">
      <Checkbox checked={checked} onCheckedChange={onToggle} className="sr-only" />
      {icon}
      <span className="font-medium text-xs sm:text-sm">{label}</span>
    </Label>
  );
}

export function SliderBlock({ label, value, min, max, onChange, help }: { label: string; value: number; min: number; max: number; onChange: (value: number) => void; help?: string }) {
  return (
    <div className="space-y-4 border-t pt-4">
      <div className="flex items-center justify-between">
        <Label className="text-sm font-medium">{label}</Label>
        <span className="text-sm font-bold text-primary">{value}</span>
      </div>
      <Slider value={[value]} onValueChange={(next) => onChange(next[0])} min={min} max={max} step={1} />
      {help ? <p className="text-xs text-muted-foreground">{help}</p> : null}
    </div>
  );
}

export function MiniSlider({ label, value, max, onChange, help }: { label: string; value: number; max: number; onChange: (value: number) => void; help: string }) {
  return (
    <div className="space-y-2">
      <div className="flex justify-between">
        <Label className="text-xs">{label}</Label>
        <span className="text-xs">{value.toFixed(1)}</span>
      </div>
      <Slider value={[value]} onValueChange={(next) => onChange(next[0])} min={0} max={max} step={0.1} />
      <p className="text-[10px] text-muted-foreground">{help}</p>
    </div>
  );
}
