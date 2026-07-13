import { ExternalLink, FileImage, FolderOpen, Image as ImageIcon, Play, ShieldAlert } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { ArtifactPathList } from "@/components/common/ArtifactPathList";
import { RuntimeDependencyPrompt } from "@/components/common/RuntimeDependencyPrompt";
import { TaskProgressBoard } from "@/components/common/TaskProgressBoard";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { useI18n } from "@/shared/i18n";
import { relativePath } from "@/shared/utils/path";
import { useImageCompressorPage } from "./hooks";

export default function ImageCompressorPage() {
  const page = useImageCompressorPage();
  const { t } = useI18n();

  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-start">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t("图片压缩")}</h1>
          <p className="mt-1 text-muted-foreground">{t("将常见图片格式及 RAW/HEIC 转换为 WebP 或 AVIF，支持单文件与整个文件夹")}</p>
        </div>
      </div>

      <RuntimeDependencyPrompt
        dependencyName="FFmpeg"
        runtime="ffmpeg"
        ready={Boolean(page.runtime?.ready)}
        loading={page.runtimeLoading}
        message={page.runtime?.ffmpegVersion ?? page.runtime?.message ?? t("正在检测 FFmpeg 运行时")}
        sourceName={page.runtime?.sourceName}
        sourceUrl={page.runtime?.sourceUrl}
        downloadSupported={page.runtime?.downloadSupported}
        refreshing={page.runtimeLoading}
        onRefresh={page.refreshRuntime}
      />

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ImageIcon className="h-5 w-5 text-primary" />
            {t("压缩配置")}
          </CardTitle>
          <CardDescription>{t("调整图片画质、特效等参数")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <Label>{t("导出质量")} ({page.quality})</Label>
              <span className="text-sm font-medium text-primary">{page.quality}%</span>
            </div>
            <Slider value={[page.quality]} onValueChange={(value) => page.setQuality(value[0])} min={1} max={100} step={1} className="py-2" />
            <div className="flex justify-between text-xs font-medium text-muted-foreground">
              <span>{t("低质量 / 极小体积")}</span>
              <span>{t("中等 (推荐 75-90)")}</span>
              <span>{t("无损质量")}</span>
            </div>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-3">
              <Label htmlFor="output-format">{t("输出格式")}</Label>
              <select
                id="output-format"
                className="flex h-9 w-full max-w-[200px] items-center justify-between rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm focus:outline-none focus:ring-1 focus:ring-ring"
                value={page.format}
                onChange={(event) => page.setFormat(event.target.value === "avif" ? "avif" : "webp")}
              >
                <option value="webp">WebP</option>
                <option value="avif">AVIF</option>
              </select>
            </div>
            <div className="space-y-3">
              <Label htmlFor="corner-r">{t("圆角半径 (px)")}</Label>
              <Input
                id="corner-r"
                type="number"
                placeholder={t("无圆角 (0)")}
                value={page.cornerRadius}
                onChange={(event) => page.setCornerRadius(event.target.value)}
                min={0}
                className="max-w-[200px]"
              />
            </div>
          </div>

          <div className="space-y-3 border-t pt-4">
            <Label>{t("选择要处理的文件或文件夹")}</Label>
            <div className="flex flex-wrap items-center gap-3">
              <Button variant={page.selectedSource?.type === "files" ? "default" : "secondary"} onClick={page.selectFiles} disabled={page.busy}>
                <FileImage className="h-4 w-4" />
                {t("选择文件")}
              </Button>
              <Button variant={page.selectedSource?.type === "folder" ? "default" : "secondary"} onClick={page.selectFolder} disabled={page.busy}>
                <FolderOpen className="h-4 w-4" />
                {t("选择文件夹")}
              </Button>
              {page.selectedSource ? (
                <Button onClick={page.startProcessing} disabled={page.busy} className="ml-auto min-w-[140px]">
                  <Play className="h-4 w-4" />
                  {t("开始处理")}
                </Button>
              ) : null}
            </div>

            {page.selectedSource ? (
              <div className="mt-2 space-y-3">
                {page.selectedSource.type === "folder" ? (
                  <div className="flex items-center space-x-2">
                    <Label className="text-sm font-medium">{t("递归深度")}</Label>
                    <Input
                      type="number"
                      min={1}
                      max={10}
                      value={page.maxDepth}
                      onChange={(event) => page.setMaxDepth(Number(event.target.value) || 1)}
                      className="h-8 w-24"
                    />
                    <span className="text-xs text-muted-foreground">{t("1 表示只处理当前文件夹，2 包含第一层子文件夹...")}</span>
                  </div>
                ) : null}

                <div className="group relative rounded-md border bg-muted p-3 font-mono text-sm">
                  {page.selectedSource.type === "folder" ? (
                    <div className="space-y-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="break-all font-semibold">{page.selectedSource.path}</span>
                        <Button variant="ghost" size="icon-xs" title={t("打开所在位置")} onClick={() => revealItemInDir(page.selectedSource!.type === "folder" ? page.selectedSource!.path : page.selectedSource!.paths[0])}>
                          <ExternalLink className="h-3 w-3" />
                        </Button>
                      </div>
                      {page.folderFilesPreview && page.folderFilesPreview.length > 0 ? (
                        <div className="mt-2 max-h-[100px] space-y-1 overflow-y-auto border-t border-muted-foreground/20 pt-2">
                          {page.folderFilesPreview.map((path) => (
                            <div key={path} className="break-all text-xs">{relativePath(page.selectedSource!.type === "folder" ? page.selectedSource!.path : "", path)}</div>
                          ))}
                        </div>
                      ) : null}
                      {page.folderFilesPreview && page.folderFilesPreview.length === 0 ? (
                        <div className="mt-2 border-t border-muted-foreground/20 pt-2 text-xs text-muted-foreground">{t("未发现支持的图片文件")}</div>
                      ) : null}
                    </div>
                  ) : (
                    <div className="max-h-[100px] space-y-1 overflow-y-auto">
                      {page.selectedSource.paths.map((path) => (
                        <div key={path} className="flex items-center justify-between gap-2">
                          <span className="break-all">{path}</span>
                          <Button variant="ghost" size="icon-xs" title={t("打开所在位置")} onClick={() => revealItemInDir(path)}>
                            <ExternalLink className="h-3 w-3" />
                          </Button>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            ) : null}
          </div>

          <div className="space-y-3 border-t pt-4">
            <Label>{t("输出目录")}</Label>
            <div className="flex gap-2">
              <Input value={page.outputDir || ""} placeholder={t("留空则与源文件同级目录")} readOnly className="cursor-not-allowed bg-muted/50 font-mono text-sm" />
              <Button variant="secondary" onClick={page.selectOutDir} disabled={page.busy} title={t("更改输出目录")}>
                <FolderOpen className="h-4 w-4" />
              </Button>
              {page.outputDir ? (
                <Button variant="outline" onClick={() => revealItemInDir(page.outputDir)} disabled={page.busy} title={t("打开输出目录")}>
                  <ExternalLink className="h-4 w-4" />
                </Button>
              ) : null}
            </div>
            {page.selectedSource ? (
              <div className="space-y-2">
                <div className="flex items-center justify-between gap-3">
                  <Label>{t("输出产物")}</Label>
                  <span className="text-xs text-muted-foreground">{t("预计 {count} 个文件", { count: page.outputArtifacts.length })}</span>
                </div>
                <ArtifactPathList paths={page.outputArtifacts} emptyMessage={t("未发现预计输出产物")} />
              </div>
            ) : null}
          </div>

          <div className="pt-2 text-xs text-muted-foreground">
            {t("支持格式：JPG · PNG · GIF · BMP · TIFF · HEIC · AVIF · RAW 等")} → <strong className="text-primary">WebP / AVIF</strong>
          </div>
        </CardContent>
      </Card>

      {page.error ? (
        <div className="flex gap-3 rounded-lg border border-destructive/20 bg-destructive/10 p-4 text-destructive">
          <ShieldAlert className="h-5 w-5 shrink-0" />
          <div className="text-sm">
            <strong className="mb-1 block font-semibold">{t("操作失败")}</strong>
            <p className="break-all">{page.error}</p>
          </div>
        </div>
      ) : null}

      <TaskProgressBoard job={page.job} onCancel={page.cancelJob} />
    </div>
  );
}
