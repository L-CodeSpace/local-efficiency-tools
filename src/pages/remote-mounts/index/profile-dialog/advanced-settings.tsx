/*
 * 核心职责：展示远程挂载推荐参数和高级参数表单。
 * 业务痛点：rclone mount 兼容性参数较多，需要默认推荐值与可自定义入口分离。
 * 能力边界：只负责高级参数 UI，不执行保存、挂载或路径选择。
 */

import { RotateCcw, Settings2 } from "lucide-react";
import type { MountAdvancedOptions } from "@/api_tauri";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { useI18n } from "@/shared/i18n";
import { type CacheMode, type useRemoteMountsPage } from "../../hooks";

type RemoteMountsPageState = ReturnType<typeof useRemoteMountsPage>;

type AdvancedMountSettingsProps = {
  page: RemoteMountsPageState;
  supportsDriveLetter: boolean;
};

export function RecommendedMountSettings({ page, supportsDriveLetter }: AdvancedMountSettingsProps) {
  const { t } = useI18n();
  const driveLetter = page.form.driveLetter.trim();
  const networkMode = supportsDriveLetter && driveLetter && page.form.advancedOptions.networkMode;
  const targetLabel = networkMode ? `${t("网络盘")} ${driveLetter}` : t("目录挂载");

  return (
    <section className="rounded-md border border-dashed bg-muted/30 p-3 text-sm md:col-span-2">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-2">
          <div className="flex items-center gap-2 font-medium">
            <Settings2 className="h-4 w-4 text-primary" />
            {t("默认推荐配置")}
          </div>
          <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-2 lg:grid-cols-4">
            <InfoItem label={t("目标")} value={targetLabel} />
            <InfoItem label={t("缓存")} value={`${page.form.cacheMode} / ${page.form.advancedOptions.vfsCacheMaxSize}`} />
            <InfoItem label={t("读取")} value={`${page.form.advancedOptions.vfsReadChunkSize} / ${page.form.advancedOptions.bufferSize}`} />
            <InfoItem label={t("超时")} value={`${page.form.advancedOptions.connectTimeout} / ${page.form.advancedOptions.ioTimeout}`} />
          </div>
        </div>
        <Button type="button" variant="outline" size="sm" onClick={page.resetAdvancedOptions}>
          <RotateCcw className="h-4 w-4" />
          {t("恢复推荐配置")}
        </Button>
      </div>
    </section>
  );
}

export function AdvancedMountSettings({ page, supportsDriveLetter }: AdvancedMountSettingsProps) {
  const { t } = useI18n();
  const options = page.form.advancedOptions;
  const canUseNetworkMode = supportsDriveLetter && Boolean(page.form.driveLetter.trim());

  return (
    <Accordion type="single" collapsible className="rounded-md border px-3 md:col-span-2">
      <AccordionItem value="advanced" className="border-b-0">
        <AccordionTrigger>{t("高级设置")}</AccordionTrigger>
        <AccordionContent>
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={t("缓存模式")}>
              <Select value={page.form.cacheMode} onValueChange={(value) => page.updateForm({ cacheMode: value as CacheMode })}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="off">off</SelectItem>
                  <SelectItem value="minimal">minimal</SelectItem>
                  <SelectItem value="writes">writes</SelectItem>
                  <SelectItem value="full">full</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <TextOption label={t("缓存上限")} value={options.vfsCacheMaxSize} onChange={(value) => updateAdvanced(page, { vfsCacheMaxSize: value })} />
            <TextOption label={t("缓存保留")} value={options.vfsCacheMaxAge} onChange={(value) => updateAdvanced(page, { vfsCacheMaxAge: value })} />
            <TextOption label={t("读取块")} value={options.vfsReadChunkSize} onChange={(value) => updateAdvanced(page, { vfsReadChunkSize: value })} />
            <TextOption label="Buffer" value={options.bufferSize} onChange={(value) => updateAdvanced(page, { bufferSize: value })} />
            <TextOption label={t("轮询间隔")} value={options.pollInterval} onChange={(value) => updateAdvanced(page, { pollInterval: value })} />
            <TextOption label={t("连接超时")} value={options.connectTimeout} onChange={(value) => updateAdvanced(page, { connectTimeout: value })} />
            <TextOption label={t("IO 超时")} value={options.ioTimeout} onChange={(value) => updateAdvanced(page, { ioTimeout: value })} />
            <NumberOption label={t("重试次数")} value={options.retries} onChange={(value) => updateAdvanced(page, { retries: value })} />
            <NumberOption label={t("低层重试")} value={options.lowLevelRetries} onChange={(value) => updateAdvanced(page, { lowLevelRetries: value })} />
            <TextOption label={t("重试间隔")} value={options.retriesSleep} onChange={(value) => updateAdvanced(page, { retriesSleep: value })} />
            <ToggleOption label="Links" checked={options.links} onChange={(links) => updateAdvanced(page, { links })} />
            <ToggleOption
              label="Network mode"
              checked={canUseNetworkMode && options.networkMode}
              disabled={!canUseNetworkMode}
              onChange={(networkMode) => updateAdvanced(page, { networkMode })}
            />
          </div>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}

function updateAdvanced(
  page: RemoteMountsPageState,
  patch: Partial<MountAdvancedOptions>,
) {
  page.updateForm({
    advancedOptions: {
      ...page.form.advancedOptions,
      ...patch,
    },
  });
}

function InfoItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <div>{label}</div>
      <div className="truncate font-mono text-foreground">{value}</div>
    </div>
  );
}

function TextOption({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return (
    <Field label={label}>
      <Input value={value} onChange={(event) => onChange(event.target.value)} />
    </Field>
  );
}

function NumberOption({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return (
    <Field label={label}>
      <Input
        type="number"
        min={0}
        max={100}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </Field>
  );
}

function ToggleOption({
  label,
  checked,
  disabled,
  onChange,
}: {
  label: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex h-10 items-center justify-between gap-3 border px-3">
      <span className={cn("text-sm", disabled && "text-muted-foreground")}>{label}</span>
      <Switch checked={checked} disabled={disabled} onCheckedChange={onChange} />
    </label>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      {children}
    </div>
  );
}
