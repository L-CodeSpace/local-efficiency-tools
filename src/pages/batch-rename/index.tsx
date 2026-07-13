import { ExternalLink, FileText, FolderOpen, Play, RefreshCw, Search, Settings2, Trash2, Type } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { TaskProgressBoard } from "@/components/common/TaskProgressBoard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useI18n } from "@/shared/i18n";
import { relativePath } from "@/shared/utils/path";
import { useBatchRenamePage } from "./hooks";

export default function BatchRenamePage() {
  const page = useBatchRenamePage();
  const { t } = useI18n();
  const selectedCount = page.items.filter((item) => item.selected).length;
  const changedCount = page.items.filter((item) => item.newName !== item.originalName).length;
  const selectableCount = page.items.filter((item) => item.newName !== item.originalName && !item.collision).length;

  return (
    <div className="flex h-full flex-col space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex shrink-0 flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t("批量重命名")}</h1>
          <p className="mt-1 text-muted-foreground">{t("使用正则表达式高级批量重命名文件")}</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={page.pickDir}>
            <FolderOpen className="h-4 w-4" /> {t("选择目录")}
          </Button>
          <Button variant="ghost" size="icon-sm" onClick={page.preview} disabled={!page.root || page.busy}>
            <RefreshCw className={`h-4 w-4 ${page.busy ? "animate-spin" : ""}`} />
          </Button>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-6">
        <div className="shrink-0 grid-cols-1 gap-6 md:grid-cols-3">
          <Card className="rounded-xl shadow-sm">
            <CardHeader className="p-5 pb-0">
              <CardTitle className="flex items-center gap-2 text-base font-semibold normal-case tracking-normal">
                <Settings2 className="h-4 w-4" /> {t("目标设置")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4 p-5">
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">{t("当前目录")}</Label>
                <div className="group flex items-center justify-between gap-2 truncate rounded-md bg-muted/50 p-2 font-mono text-sm" title={page.root || t("未选择")}>
                  <span className="truncate">{page.root || t("未选择")}</span>
                  {page.root ? (
                    <Button variant="ghost" size="icon-xs" title={t("打开所在位置")} onClick={() => revealItemInDir(page.root)}>
                      <ExternalLink className="h-3 w-3" />
                    </Button>
                  ) : null}
                </div>
              </div>
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">{t("递归深度")}</Label>
                <Input type="number" min={1} max={10} value={page.maxDepth} onChange={(event) => page.setMaxDepth(Number(event.target.value) || 1)} />
              </div>
              <div className="space-y-3 border-t pt-3">
                <label className="flex items-center space-x-2">
                  <Checkbox checked={page.autoResolveCollision} onCheckedChange={(checked) => page.setAutoResolveCollision(checked === true)} />
                  <span className="text-xs font-medium">{t("自动解决冲突 (添加数字后缀)")}</span>
                </label>
                {page.autoResolveCollision ? (
                  <div className="flex items-center space-x-2 pl-6">
                    <Label className="whitespace-nowrap text-xs text-muted-foreground">{t("起始序号")}</Label>
                    <Input type="number" min={0} className="h-8 w-24 text-xs" value={page.collisionStartIndex} onChange={(event) => page.setCollisionStartIndex(Number(event.target.value) || 0)} />
                  </div>
                ) : null}
                <label className="flex items-center space-x-2">
                  <Checkbox checked={page.preserveExtension} onCheckedChange={(checked) => page.setPreserveExtension(checked === true)} />
                  <span className="text-xs font-medium">{t("保留原文件后缀名")}</span>
                </label>
              </div>
            </CardContent>
          </Card>

          <Card className="rounded-xl shadow-sm">
            <CardHeader className="p-5 pb-0">
              <CardTitle className="flex items-center gap-2 text-base font-semibold normal-case tracking-normal">
                <Type className="h-4 w-4" /> {t("预设规则")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4 p-5">
              <div className="grid grid-cols-1 gap-2">
                <Button variant="secondary" size="sm" className="justify-start" onClick={() => page.applyPreset("(.+)", "$1-副本")}>{t("添加后缀: name -> name-副本")}</Button>
                <Button variant="secondary" size="sm" className="justify-start" onClick={() => page.applyPreset("(.+)\\.(.+)$", "$1-$INDEX.$2")}>{t("序列命名: name-1, name-2")}</Button>
                <Button variant="secondary" size="sm" className="justify-start" onClick={() => page.applyPreset("oldName", "newName")}>{t("简单替换: oldName -> newName")}</Button>
              </div>
            </CardContent>
          </Card>

          <Card className="rounded-xl shadow-sm">
            <CardHeader className="p-5 pb-0">
              <CardTitle className="flex items-center gap-2 text-base font-semibold normal-case tracking-normal">
                <Search className="h-4 w-4" /> {t("正则表达式规则")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4 p-5">
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">{t("查找 (Regex)")}</Label>
                <Input placeholder={t("例如: (.*)\\.png$")} value={page.pattern} onChange={(event) => page.setPattern(event.target.value)} className="font-mono text-sm" />
              </div>
              <div className="space-y-2">
                <Label className="text-xs text-muted-foreground">{t("替换为")}</Label>
                <Input placeholder={t("例如: $1-compress.png")} value={page.replacement} onChange={(event) => page.setReplacement(event.target.value)} className="font-mono text-sm" />
                <p className="mt-1 text-[10px] text-muted-foreground">{t("支持 $1, $2 分组，使用 $INDEX 插入自增序号")}</p>
              </div>
            </CardContent>
          </Card>
        </div>

        <div className="flex min-h-[400px] flex-1 flex-col overflow-hidden rounded-xl border bg-card text-card-foreground shadow-sm">
          <div className="flex shrink-0 items-center justify-between border-b bg-muted/20 p-4">
            <h2 className="flex items-center gap-2 font-semibold">
              <FileText className="h-4 w-4" /> {t("预览与执行")}
            </h2>
            <div className="flex items-center gap-3">
              <span className="text-sm text-muted-foreground">{t("已选")}: {selectedCount} / {t("变更")}: {changedCount}</span>
              <Button onClick={page.preview} variant="outline" disabled={page.busy || !page.root || !page.pattern}>
                {t("生成预览")}
              </Button>
              <Button onClick={page.execute} disabled={page.busy || selectedCount === 0 || !page.root}>
                <Play className="h-4 w-4" />
                {t("开始重命名")}
              </Button>
            </div>
          </div>
          {page.busy ? (
            <div className="shrink-0 border-b bg-primary/5 p-4">
              <div className="mb-2 flex items-center justify-between text-xs"><span>{t("处理中")}</span><span>...</span></div>
              <Progress value={60} className="h-2" />
            </div>
          ) : null}
          <div className="max-h-[calc(100vh-220px)] flex-1 overflow-auto">
            {page.items.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center p-6 text-center text-muted-foreground opacity-50">
                <Search className="mb-4 h-12 w-12" />
                <p>{t("未选择文件夹或文件夹内无文件")}</p>
              </div>
            ) : (
              <Table>
                <TableHeader className="sticky top-0 z-10 bg-card shadow-sm">
                  <TableRow>
                    <TableHead className="w-12 text-center">
                      <Checkbox checked={selectedCount > 0 && selectedCount === selectableCount} onCheckedChange={(checked) => page.toggleAll(checked === true)} disabled={selectableCount === 0 || page.busy} />
                    </TableHead>
                    <TableHead className="w-[45%]">{t("新名称")}</TableHead>
                    <TableHead className="w-[45%]">{t("原名称")}</TableHead>
                    <TableHead className="w-24 text-center">{t("状态")}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {page.items.map((item) => {
                    const changed = item.newName !== item.originalName;
                    return (
                      <TableRow key={item.originalPath} className={item.collision ? "bg-destructive/10 hover:bg-destructive/15" : changed ? "bg-primary/5 hover:bg-primary/10" : ""}>
                        <TableCell className="text-center">
                          <Checkbox checked={item.selected} onCheckedChange={() => page.toggleItem(item.originalPath)} disabled={!changed || page.busy || item.collision} />
                        </TableCell>
                        <TableCell className="break-all font-mono text-xs">
                          {changed ? <span className={item.collision ? "font-semibold text-destructive" : "font-semibold text-primary"}>{item.newName}</span> : <span className="opacity-40">-</span>}
                          {item.collision ? <div className="mt-1 flex items-center gap-1 text-[10px] text-destructive"><Trash2 className="h-3 w-3" /> {t("命名冲突")}</div> : null}
                        </TableCell>
                        <TableCell className="break-all font-mono text-xs opacity-70">{relativePath(page.root, item.originalPath)}</TableCell>
                        <TableCell className="text-center text-xs">
                          {changed && !item.collision ? <Badge variant="outline">{t("待处理")}</Badge> : null}
                          {item.autoResolved ? <Badge variant="secondary" className="mt-1 scale-90 whitespace-nowrap text-[10px]">{t("已追加序号")}</Badge> : null}
                          {!changed ? <span className="opacity-40">-</span> : null}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            )}
          </div>
        </div>
      </div>

      {page.error ? <div className="text-sm text-destructive">{page.error}</div> : null}
      <TaskProgressBoard job={page.job} onCancel={page.cancelJob} />
    </div>
  );
}
