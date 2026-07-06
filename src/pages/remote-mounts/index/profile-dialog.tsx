/*
 * 核心职责：展示远程挂载配置弹窗。
 * 业务痛点：协议表单和挂载点高级设置会让列表页面入口过度膨胀。
 * 能力边界：只负责表单 UI，保存、选择文件和状态变更仍由页面 hook 提供。
 */

import { FolderOpen, KeyRound } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import type { MountUiContext } from "@/api_tauri";
import { cn } from "@/lib/utils";
import { useI18n } from "@/shared/i18n";
import { useRemoteMountsPage } from "../hooks";
import { AdvancedMountSettings, RecommendedMountSettings } from "./profile-dialog/advanced-settings";

type ProfileDialogProps = {
  page: ReturnType<typeof useRemoteMountsPage>;
  supportsDriveLetter: boolean;
};

export function ProfileDialog({ page, supportsDriveLetter }: ProfileDialogProps) {
  const { t } = useI18n();
  return (
      <Dialog open={page.dialogOpen} onOpenChange={page.setDialogOpen}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>{page.form.id ? t("编辑挂载") : t("新建挂载")}</DialogTitle>
            <DialogDescription>{t("密码会明文保存到 profiles.json。")}</DialogDescription>
          </DialogHeader>

          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <Field label={t("名称")}>
              <Input value={page.form.name} onChange={(event) => page.updateForm({ name: event.target.value })} placeholder={t("如：素材 FTP")} />
            </Field>
            <Field label={t("协议")}>
              <Select value={page.form.protocol} onValueChange={(value) => page.updateForm({ protocol: value as typeof page.form.protocol })}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ftp">FTP</SelectItem>
                  <SelectItem value="sftp">SFTP</SelectItem>
                  <SelectItem value="webdav">WebDAV</SelectItem>
                </SelectContent>
              </Select>
            </Field>

            {page.form.protocol === "webdav" ? (
              <>
                <Field label="WebDAV URL" className="md:col-span-2">
                  <Input value={page.form.url} onChange={(event) => page.updateForm({ url: event.target.value })} placeholder="https://example.com/dav" />
                </Field>
                <Field label={t("服务类型")}>
                  <Select value={page.form.vendor} onValueChange={(value) => page.updateForm({ vendor: value })}>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="other">Other</SelectItem>
                      <SelectItem value="nextcloud">Nextcloud</SelectItem>
                      <SelectItem value="owncloud">ownCloud</SelectItem>
                      <SelectItem value="sharepoint">SharePoint</SelectItem>
                    </SelectContent>
                  </Select>
                </Field>
              </>
            ) : (
              <>
                <Field label={t("主机")}>
                  <Input value={page.form.host} onChange={(event) => page.updateForm({ host: event.target.value })} placeholder="example.com" />
                </Field>
                <Field label={t("端口")}>
                  <Input
                    type="number"
                    min={1}
                    max={65535}
                    value={page.form.port}
                    onChange={(event) => page.updateForm({ port: event.target.value })}
                    placeholder={page.form.protocol === "sftp" ? "22" : "21"}
                  />
                </Field>
              </>
            )}

            <Field label={t("用户名")}>
              <Input value={page.form.username} onChange={(event) => page.updateForm({ username: event.target.value })} autoComplete="username" />
            </Field>
            <Field label={t("密码")}>
              <Input
                type="text"
                value={page.form.password}
                onChange={(event) => page.updateForm({ password: event.target.value })}
                autoComplete="new-password"
              />
            </Field>

            {page.form.protocol === "sftp" ? (
              <Field label={t("密钥文件")} className="md:col-span-2">
                <div className="flex gap-2">
                  <Input value={page.form.keyFile} onChange={(event) => page.updateForm({ keyFile: event.target.value })} placeholder={t("可选")} />
                  <Button type="button" variant="outline" onClick={page.pickKeyFile}>
                    <KeyRound className="h-4 w-4" />
                    {t("选择")}
                  </Button>
                </div>
              </Field>
            ) : null}

            {page.form.protocol === "ftp" ? (
              <Field label="TLS">
                <Select value={page.form.tlsMode} onValueChange={(value) => page.updateForm({ tlsMode: value })}>
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">{t("关闭")}</SelectItem>
                    <SelectItem value="explicit">{t("显式 TLS")}</SelectItem>
                    <SelectItem value="implicit">{t("隐式 TLS")}</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            ) : null}

            <Field label={t("远程路径")}>
              <Input value={page.form.remotePath} onChange={(event) => page.updateForm({ remotePath: event.target.value })} placeholder={t("/ 或目录路径")} />
            </Field>

            <RecommendedMountSettings page={page} supportsDriveLetter={supportsDriveLetter} />

            {supportsDriveLetter ? (
              <Field label={t("Windows 盘符")}>
                <Input
                  value={page.form.driveLetter}
                  onChange={(event) => page.updateForm({ driveLetter: event.target.value })}
                  placeholder={page.uiContext?.defaultDriveLetter ?? t("如 Z:")}
                />
              </Field>
            ) : null}
            <Field label={supportsDriveLetter ? t("本地目录（兼容选项）") : t("挂载到本地目录")} className={supportsDriveLetter ? "" : "md:col-span-2"}>
              <div className="flex gap-2">
                <Input
                  value={page.form.mountPoint}
                  onChange={(event) => page.updateForm({ mountPoint: event.target.value })}
                  placeholder={defaultMountPlaceholder(page.uiContext)}
                />
                <Button type="button" variant="outline" onClick={page.pickMountPoint}>
                  <FolderOpen className="h-4 w-4" />
                  {t("选择")}
                </Button>
              </div>
            </Field>

            <AdvancedMountSettings page={page} supportsDriveLetter={supportsDriveLetter} />

            <div className="grid gap-3 md:col-span-2 sm:grid-cols-3">
              <ToggleBox label={t("只读挂载")} checked={page.form.readOnly} onChange={(checked) => page.updateForm({ readOnly: checked })} />
              <ToggleBox label={t("跳过证书校验")} checked={page.form.noCheckCertificate} onChange={(checked) => page.updateForm({ noCheckCertificate: checked })} />
              <ToggleBox label={t("保存后启用")} checked={page.form.enabled} onChange={(checked) => page.updateForm({ enabled: checked })} />
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => page.setDialogOpen(false)}>{t("取消")}</Button>
            <Button onClick={page.saveProfile} disabled={page.saving}>{page.saving ? `${t("保存中")}...` : t("保存配置")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
  );
}

function defaultMountPlaceholder(context: MountUiContext | null) {
  if (!context) return "";
  if (context.platform === "windows") {
    return context.defaultDriveLetter ? "留空则使用推荐盘符" : context.defaultMountExample;
  }
  return context.defaultMountExample;
}

function Field({ label, className, children }: { label: string; className?: string; children: React.ReactNode }) {
  return (
    <div className={cn("space-y-2", className)}>
      <Label>{label}</Label>
      {children}
    </div>
  );
}

function ToggleBox({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return (
    <label className="flex items-center justify-between gap-3 border p-3">
      <span className="text-sm">{label}</span>
      <Switch checked={checked} onCheckedChange={onChange} />
    </label>
  );
}
