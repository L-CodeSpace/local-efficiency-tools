/*
 * 核心职责：应用设置页面路由入口。
 * 业务痛点：设置页需要聚合外观、导航、后台和系统信息，运行时资源操作不应挤在入口文件。
 * 能力边界：只负责页面装配和设置状态绑定。
 */

import { useEffect, useRef } from "react";
import { Activity, Cpu, Database, HardDrive, LayoutTemplate, Monitor, Moon, Power, Sun, Wrench } from "lucide-react";
import { useSearchParams } from "react-router-dom";
import { appNavItems } from "@/components/app/nav";
import { useTheme } from "@/components/app/ThemeProvider";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { toggleHiddenNavItem, useHiddenNavItems } from "@/shared/state/navVisibility";
import { languageOptions, useI18n } from "@/shared/i18n";
import { useSettingsPage } from "./hooks";
import { RuntimeResourceLine } from "./index/runtime-resource-line";

export default function SettingsPage() {
  const { theme, setTheme } = useTheme();
  const { language, setLanguage, t } = useI18n();
  const hiddenNavItems = useHiddenNavItems();
  const page = useSettingsPage();
  const [searchParams] = useSearchParams();
  const runtimeTarget = normalizeRuntimeTarget(searchParams.get("runtime"));
  const runtimeSectionRef = useRef<HTMLDivElement>(null);
  const ffmpegLineRef = useRef<HTMLDivElement>(null);
  const rcloneLineRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!runtimeTarget) return;
    const timer = window.setTimeout(() => {
      const target = runtimeTarget === "ffmpeg" ? ffmpegLineRef.current : rcloneLineRef.current;
      (target ?? runtimeSectionRef.current)?.scrollIntoView({ behavior: "smooth", block: "center" });
    }, 120);
    return () => window.clearTimeout(timer);
  }, [runtimeTarget, page.mediaRuntime, page.mountRuntime]);

  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-start">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t("应用设置")}</h1>
          <p className="mt-1 text-muted-foreground">{t("管理应用程序的界面与偏好设置")}</p>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("外观设置")}</CardTitle>
          <CardDescription>{t("自定义应用程序的颜色主题")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-4">
            <Label>{t("界面主题")}</Label>
            <div className="grid max-w-2xl grid-cols-1 gap-4 sm:grid-cols-3">
              <ThemeCard active={theme === "light"} icon={<Sun className={`mb-3 h-8 w-8 ${theme === "light" ? "text-primary" : "text-muted-foreground"}`} />} label={t("浅色模式")} onClick={() => setTheme("light")} />
              <ThemeCard active={theme === "dark"} icon={<Moon className={`mb-3 h-8 w-8 ${theme === "dark" ? "text-primary" : "text-muted-foreground"}`} />} label={t("深色模式")} onClick={() => setTheme("dark")} />
              <ThemeCard active={theme === "system"} icon={<Monitor className={`mb-3 h-8 w-8 ${theme === "system" ? "text-primary" : "text-muted-foreground"}`} />} label={t("跟随系统")} onClick={() => setTheme("system")} />
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("语言设置")}</CardTitle>
          <CardDescription>{t("选择界面显示语言")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="max-w-sm space-y-2">
            <Label>{t("界面语言")}</Label>
            <Select value={language} onValueChange={(value) => setLanguage(value as typeof language)}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {languageOptions.map((option) => (
                  <SelectItem key={option.code} value={option.code}>
                    {t(option.label)} · {option.nativeLabel}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <LayoutTemplate className="h-5 w-5 text-primary" />
            {t("侧边栏显示")}
          </CardTitle>
          <CardDescription>{t("控制哪些功能入口在左侧边栏显示，简化您的工作区")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-3">
            {appNavItems.filter((item) => item.id !== "settings").map((item) => {
              const Icon = item.icon;
              return (
                <div key={item.id} className="flex items-center justify-between rounded-xl border p-4 transition-colors hover:bg-muted/50">
                  <div className="flex items-center gap-3">
                    <Icon className="h-5 w-5 text-primary" />
                    <div>
                      <div className="text-sm font-medium">{t(item.label)}</div>
                      <div className="text-xs text-muted-foreground">{t(item.description)}</div>
                    </div>
                  </div>
                  <Switch checked={!hiddenNavItems.includes(item.id)} onCheckedChange={() => toggleHiddenNavItem(item.id)} />
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Power className="h-5 w-5 text-primary" />
            {t("后台运行")}
          </CardTitle>
          <CardDescription>{t("控制关闭窗口时是否隐藏到托盘并保留正在运行的挂载")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex max-w-2xl items-center justify-between border p-4 transition-colors hover:bg-muted/50">
            <div>
              <div className="text-sm font-medium">{t("关闭窗口后继续运行")}</div>
              <div className="mt-1 text-xs text-muted-foreground">{t("有活动挂载时仍会自动进入托盘，避免直接断开映射。")}</div>
            </div>
            <Switch checked={page.backgroundEnabled} disabled={page.backgroundSaving} onCheckedChange={page.setBackground} />
          </div>
        </CardContent>
      </Card>

      <div ref={runtimeSectionRef}>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Wrench className="h-5 w-5 text-primary" />
              {t("运行时资源")}
            </CardTitle>
            <CardDescription>{t("查看媒体处理和远程挂载依赖的当前可执行文件位置")}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <RuntimeResourceLine
              lineRef={ffmpegLineRef}
              highlighted={runtimeTarget === "ffmpeg"}
              name="FFmpeg"
              status={page.mediaRuntime?.ready ? t("已就绪") : page.mediaRuntime ? t("未找到") : t("检测中")}
              ready={Boolean(page.mediaRuntime?.ready)}
              detail={page.mediaRuntime?.ffmpegVersion ?? page.mediaRuntime?.message ?? t("正在检测 FFmpeg")}
              path={page.mediaRuntime?.path}
              emptyPath={t("未检测到 FFmpeg")}
              sourceName={page.mediaRuntime?.sourceName}
              sourceUrl={page.mediaRuntime?.sourceUrl}
              downloadSupported={Boolean(page.mediaRuntime?.downloadSupported)}
              downloading={page.runtimeDownloading === "ffmpeg"}
              canOpen={Boolean(page.mediaRuntime?.ready && page.mediaRuntime.path)}
              onDownload={() => page.downloadRuntime("ffmpeg")}
            />
            <RuntimeResourceLine
              lineRef={rcloneLineRef}
              highlighted={runtimeTarget === "rclone"}
              name="rclone"
              status={page.mountRuntime?.installed ? t("已安装") : page.mountRuntime ? t("待下载") : t("检测中")}
              ready={Boolean(page.mountRuntime?.installed)}
              detail={page.mountRuntime?.version ?? (page.mountRuntime ? `${t("期望版本")}: ${page.mountRuntime.expectedVersion}` : t("正在检测 rclone"))}
              path={page.mountRuntime?.path}
              emptyPath={t("尚未获取 rclone 路径")}
              sourceName={page.mountRuntime?.sourceName}
              sourceUrl={page.mountRuntime?.sourceUrl}
              downloadSupported={Boolean(page.mountRuntime?.downloadSupported)}
              downloading={page.runtimeDownloading === "rclone"}
              canOpen={Boolean(page.mountRuntime?.installed && page.mountRuntime.path)}
              onDownload={() => page.downloadRuntime("rclone")}
            />
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("系统信息")}</CardTitle>
          <CardDescription>{t("当前设备的硬件与系统配置状态")}</CardDescription>
        </CardHeader>
        <CardContent>
          {page.error ? <div className="text-sm text-destructive">{page.error}</div> : null}
          {page.hardware ? (
            <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
              <div className="space-y-4">
                <InfoLine icon={<Monitor className="mt-0.5 h-5 w-5 text-primary" />} title={t("操作系统")} lines={[`${page.hardware.osName} ${page.hardware.osVersion}`, `${t("主机名")}: ${page.hardware.hostname}`]} />
                <InfoLine icon={<Cpu className="mt-0.5 h-5 w-5 text-primary" />} title={t("处理器 (CPU)")} lines={[page.hardware.cpuName, `${page.hardware.cpuCores} ${t("核心")}`]} />
                <InfoLine icon={<Database className="mt-0.5 h-5 w-5 text-primary" />} title={t("主板")} lines={[page.hardware.motherboard]} />
              </div>
              <div className="space-y-4">
                <InfoLine icon={<HardDrive className="mt-0.5 h-5 w-5 text-primary" />} title={t("内存 (RAM)")} lines={[`${t("总计")}: ${formatGb(page.hardware.ramTotal)}`, `${t("已用")}: ${formatGb(page.hardware.ramUsed)}`, `${t("交换空间")}: ${formatGb(page.hardware.swapTotal)} (${t("已用")} ${formatGb(page.hardware.swapUsed)})`]} />
                <InfoLine icon={<Activity className="mt-0.5 h-5 w-5 text-primary" />} title={t("显卡 (GPU)")} lines={page.hardware.gpuInfo.length > 0 ? page.hardware.gpuInfo.map((gpu) => `${gpu.name} · ${gpu.vram}`) : [t("未获取到显卡信息")]} />
              </div>
            </div>
          ) : (
            <div className="py-4 text-center text-sm text-muted-foreground">{t("正在加载系统信息")}...</div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function normalizeRuntimeTarget(value: string | null) {
  if (value === "ffmpeg" || value === "rclone") return value;
  return null;
}

function ThemeCard({ active, icon, label, onClick }: { active: boolean; icon: React.ReactNode; label: string; onClick: () => void }) {
  return (
    <button onClick={onClick} className={`flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 p-6 transition-colors hover:bg-accent ${active ? "border-primary bg-primary/5" : "border-border"}`}>
      {icon}
      <span className={`font-medium ${active ? "text-primary" : ""}`}>{label}</span>
    </button>
  );
}

function InfoLine({ icon, title, lines }: { icon: React.ReactNode; title: string; lines: string[] }) {
  return (
    <div className="flex items-start gap-3">
      {icon}
      <div>
        <div className="font-medium">{title}</div>
        {lines.map((line) => (
          <div key={line} className="text-sm text-muted-foreground">{line}</div>
        ))}
      </div>
    </div>
  );
}

function formatGb(bytes: number) {
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

