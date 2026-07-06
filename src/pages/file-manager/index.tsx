/*
 * 核心职责：文件管理页面路由入口。
 * 业务痛点：文件列表、编辑弹窗和路径工具塞在一起会形成维护负担。
 * 能力边界：只负责页面状态和文件操作 UI 装配。
 */

import { useState } from "react";
import { ArrowUp, Edit, FilePlus, FileText, FolderOpen, FolderPlus, Home, RefreshCw, Save, Settings, Trash2, Type, X } from "lucide-react";
import { toast } from "sonner";
import { FileIcon, isTextEntry } from "@/components/common/FileIcon";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { type FileEntry } from "@/api_tauri";
import { logError } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";
import { formatBytes, formatDate, joinPath } from "@/shared/utils/path";
import { useFileManagerPage } from "./hooks";
import { buildCrumbs } from "./index/breadcrumbs";
import type { Modal } from "./index/types";

export default function FileManagerPage() {
  const page = useFileManagerPage();
  const [modal, setModal] = useState<Modal>({ kind: "none" });
  const [inputVal, setInputVal] = useState("");
  const [editContent, setEditContent] = useState("");
  const [saving, setSaving] = useState(false);

  const openModal = (next: Modal, value = "") => {
    setInputVal(value);
    setModal(next);
  };

  const handleOpen = async (entry: FileEntry) => {
    if (entry.isDir) {
      await page.navigate(entry.path);
      return;
    }
    if (!isTextEntry(entry)) return;
    try {
      const content = await page.readText(entry);
      setEditContent(content);
      setModal({ kind: "edit", entry, content });
    } catch (err) {
      const message = formatError(err);
      toast.error(message);
      logError(message);
    }
  };

  const handleRename = async () => {
    if (modal.kind !== "rename") return;
    const name = inputVal.trim();
    if (!name) return;
    await runAction(() => page.executeOperation({ kind: "rename", path: modal.entry.path, newName: name }), `已重命名为 ${name}`);
  };

  const handleNewFile = async () => {
    const name = inputVal.trim();
    if (!name) return;
    await runAction(() => page.executeOperation({ kind: "createFile", path: joinPath(page.path, name) }), `已创建 ${name}`);
  };

  const handleNewDir = async () => {
    const name = inputVal.trim();
    if (!name) return;
    await runAction(() => page.executeOperation({ kind: "createDir", path: joinPath(page.path, name) }), `已创建文件夹 ${name}`);
  };

  const handleDelete = async () => {
    if (modal.kind !== "delete") return;
    await runAction(
      () => page.executeOperation({ kind: "delete", path: modal.entry.path, recursive: modal.entry.isDir }),
      `已删除 ${modal.entry.name}`,
    );
  };

  const handleSaveEdit = async () => {
    if (modal.kind !== "edit") return;
    setSaving(true);
    try {
      await page.executeOperation({ kind: "writeText", path: modal.entry.path, content: editContent });
      toast.success(`已保存 ${modal.entry.name}`);
      setModal({ kind: "none" });
    } catch (err) {
      const message = formatError(err);
      toast.error(message);
      logError(message);
    } finally {
      setSaving(false);
    }
  };

  const runAction = async (action: () => Promise<void>, success: string) => {
    try {
      await action();
      toast.success(success);
      setModal({ kind: "none" });
    } catch (err) {
      const message = formatError(err);
      toast.error(message);
      logError(message);
    }
  };

  const crumbs = buildCrumbs(page.path);

  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">文件管理</h1>
          <p className="mt-1 text-muted-foreground">浏览、编辑、创建和删除文件与目录</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={page.pickDir}>
            <FolderOpen className="h-4 w-4" /> 选择目录
          </Button>
          {page.exeDir ? (
            <Button variant="outline" size="sm" onClick={() => page.navigate(page.exeDir)} title={page.exeDir}>
              <Settings className="h-4 w-4" /> 程序目录
            </Button>
          ) : null}
          {page.cwd ? (
            <Button variant="outline" size="sm" onClick={() => page.navigate(page.cwd)} title={page.cwd}>
              <Home className="h-4 w-4" /> 工作目录
            </Button>
          ) : null}
        </div>
      </div>

      {(page.exeDir || page.cwd) ? (
        <div className="flex flex-wrap gap-3">
          {page.exeDir ? <Badge variant="secondary" className="font-mono text-xs font-normal" title={page.exeDir}>程序路径 {page.exeDir}</Badge> : null}
          {page.cwd ? <Badge variant="secondary" className="font-mono text-xs font-normal" title={page.cwd}>工作目录 {page.cwd}</Badge> : null}
        </div>
      ) : null}

      <div className="overflow-hidden rounded-xl border bg-card text-card-foreground shadow-sm">
        <div className="flex flex-col justify-between gap-4 border-b bg-muted/20 p-4 sm:flex-row sm:items-center">
          <div className="flex flex-1 items-center gap-1.5 overflow-x-auto">
            <Button variant="ghost" size="icon-sm" onClick={() => crumbs.length > 1 && page.navigate(crumbs[crumbs.length - 2].path)} disabled={crumbs.length <= 1}>
              <ArrowUp className="h-4 w-4" />
            </Button>
            <div className="mx-1 h-4 w-px shrink-0 bg-border" />
            <div className="flex flex-nowrap whitespace-nowrap text-sm font-medium">
              {crumbs.map((crumb, index) => (
                <span key={`${crumb.path}-${index}`} className="flex items-center">
                  {index > 0 ? <span className="mx-1.5 text-muted-foreground/50">/</span> : null}
                  <button className="underline-offset-4 transition-colors hover:text-primary hover:underline" onClick={() => page.navigate(crumb.path)}>
                    {crumb.label}
                  </button>
                </span>
              ))}
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => openModal({ kind: "new_file" })} disabled={!page.path}>
              <FilePlus className="h-4 w-4" /> 新建文件
            </Button>
            <Button variant="outline" size="sm" onClick={() => openModal({ kind: "new_dir" })} disabled={!page.path}>
              <FolderPlus className="h-4 w-4" /> 新建文件夹
            </Button>
            <Button variant="ghost" size="icon-sm" onClick={() => page.navigate(page.path)} disabled={!page.path || page.loading}>
              <RefreshCw className={`h-4 w-4 ${page.loading ? "animate-spin" : ""}`} />
            </Button>
          </div>
        </div>

        <div className="min-h-[400px] w-full overflow-x-auto">
          {page.entries.length === 0 && !page.error && !page.loading ? (
            <div className="flex h-[400px] flex-col items-center justify-center text-muted-foreground">
              <FolderOpen className="mb-4 h-12 w-12 opacity-20" />
              <p>当前目录为空</p>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead className="w-[50%]">名称</TableHead>
                  <TableHead>大小</TableHead>
                  <TableHead>修改时间</TableHead>
                  <TableHead className="text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {page.entries.map((entry) => (
                  <TableRow key={entry.path} className="group">
                    <TableCell className="font-medium">
                      <div className="flex items-center gap-3">
                        <FileIcon entry={entry} />
                        <button
                          className={`max-w-[400px] truncate underline-offset-4 hover:underline ${entry.isDir ? "text-primary" : "text-foreground"}`}
                          onClick={() => handleOpen(entry)}
                          title={entry.path}
                        >
                          {entry.name}
                        </button>
                        {entry.readonly ? <Badge variant="outline" className="h-5 px-1.5 text-[10px] opacity-70">只读</Badge> : null}
                      </div>
                    </TableCell>
                    <TableCell className="font-mono text-xs text-muted-foreground">{entry.isDir ? "-" : formatBytes(entry.size)}</TableCell>
                    <TableCell className="font-mono text-xs text-muted-foreground">{formatDate(entry.modifiedAt)}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                        {isTextEntry(entry) ? (
                          <Button variant="ghost" size="icon-sm" className="text-blue-500 hover:bg-blue-50 hover:text-blue-600" onClick={() => handleOpen(entry)} title="编辑">
                            <Edit className="h-4 w-4" />
                          </Button>
                        ) : null}
                        <Button variant="ghost" size="icon-sm" className="text-amber-500 hover:bg-amber-50 hover:text-amber-600" onClick={() => openModal({ kind: "rename", entry }, entry.name)} title="重命名">
                          <Type className="h-4 w-4" />
                        </Button>
                        <Button variant="ghost" size="icon-sm" className="text-destructive hover:bg-destructive/10 hover:text-destructive" onClick={() => openModal({ kind: "delete", entry })} title="删除">
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </div>
      </div>

      {page.error ? <div className="text-sm text-destructive">{page.error}</div> : null}

      <Dialog open={modal.kind !== "none" && modal.kind !== "edit"} onOpenChange={(open) => !open && setModal({ kind: "none" })}>
        <DialogContent className="sm:max-w-md">
          {modal.kind === "delete" ? (
            <>
              <DialogHeader>
                <DialogTitle>确认删除</DialogTitle>
                <DialogDescription>
                  {modal.entry.isDir ? `将永久删除文件夹「${modal.entry.name}」及其所有内容，此操作不可撤销。` : `将永久删除文件「${modal.entry.name}」，此操作不可撤销。`}
                </DialogDescription>
              </DialogHeader>
              <DialogFooter className="mt-4">
                <Button variant="outline" onClick={() => setModal({ kind: "none" })}>取消</Button>
                <Button variant="destructive" onClick={handleDelete}>确认删除</Button>
              </DialogFooter>
            </>
          ) : null}
          {modal.kind === "rename" ? (
            <>
              <DialogHeader><DialogTitle>重命名</DialogTitle></DialogHeader>
              <div className="py-4"><Input autoFocus value={inputVal} onChange={(event) => setInputVal(event.target.value)} onKeyDown={(event) => event.key === "Enter" && handleRename()} /></div>
              <DialogFooter><Button variant="outline" onClick={() => setModal({ kind: "none" })}>取消</Button><Button onClick={handleRename} disabled={!inputVal.trim()}>确认</Button></DialogFooter>
            </>
          ) : null}
          {modal.kind === "new_file" || modal.kind === "new_dir" ? (
            <>
              <DialogHeader><DialogTitle>{modal.kind === "new_file" ? "新建文件" : "新建文件夹"}</DialogTitle></DialogHeader>
              <div className="py-4"><Input autoFocus placeholder={modal.kind === "new_file" ? "如：hello.txt" : "如：my-folder"} value={inputVal} onChange={(event) => setInputVal(event.target.value)} onKeyDown={(event) => event.key === "Enter" && (modal.kind === "new_file" ? handleNewFile() : handleNewDir())} /></div>
              <DialogFooter><Button variant="outline" onClick={() => setModal({ kind: "none" })}>取消</Button><Button onClick={modal.kind === "new_file" ? handleNewFile : handleNewDir} disabled={!inputVal.trim()}>创建</Button></DialogFooter>
            </>
          ) : null}
        </DialogContent>
      </Dialog>

      <Dialog open={modal.kind === "edit"} onOpenChange={(open) => !open && setModal({ kind: "none" })}>
        <DialogContent className="flex h-[90vh] w-full max-w-[90vw] flex-col overflow-hidden p-0">
          {modal.kind === "edit" ? (
            <>
              <div className="flex shrink-0 items-center justify-between border-b bg-muted/10 p-4">
                <DialogTitle className="flex items-center gap-2 font-mono text-lg normal-case tracking-normal">
                  <FileText className="h-5 w-5 text-primary" />
                  {modal.entry.name}
                </DialogTitle>
                <div className="flex gap-2">
                  <Button variant="outline" size="sm" onClick={() => setModal({ kind: "none" })}><X className="h-4 w-4" /> 关闭</Button>
                  <Button size="sm" onClick={handleSaveEdit} disabled={saving}><Save className="h-4 w-4" /> {saving ? "保存中..." : "保存"}</Button>
                </div>
              </div>
              <div className="relative flex-1 overflow-hidden bg-background p-0">
                <Textarea className="absolute inset-0 h-full w-full resize-none rounded-none border-0 p-4 font-mono text-sm leading-relaxed focus-visible:ring-0" value={editContent} onChange={(event) => setEditContent(event.target.value)} spellCheck={false} />
              </div>
            </>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}
