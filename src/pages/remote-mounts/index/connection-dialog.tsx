/*
 * 核心职责：编辑 NAS 连接与 SMB/FTP 自动选择参数。
 * 业务痛点：连接凭据必须与具体挂载目录解耦，避免重复维护同一账号。
 * 能力边界：只渲染表单并调用页面 hook。
 */

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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/shared/i18n";
import type { useRemoteMountsPage } from "../hooks";

type Page = ReturnType<typeof useRemoteMountsPage>;

export function ConnectionDialog({ page }: { page: Page }) {
  const { t } = useI18n();
  const form = page.connectionForm;
  return (
    <Dialog open={page.connectionDialogOpen} onOpenChange={page.setConnectionDialogOpen}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{form.id ? t("编辑连接") : t("新建连接")}</DialogTitle>
          <DialogDescription>{t("连接只保存服务器和凭据，远端目录将在探测后创建为工作区。")}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 md:grid-cols-2">
          <Field label={t("连接名称")}>
            <Input value={form.name} onChange={(event) => page.updateConnectionForm({ name: event.target.value })} placeholder={t("如：办公室 NAS")} />
          </Field>
          <Field label={t("传输策略")}>
            <Select value={form.transportPreference} onValueChange={(value) => page.updateConnectionForm({ transportPreference: value as typeof form.transportPreference })}>
              <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="auto">{t("自动：SMB 优先，FTP 回退")}</SelectItem>
                <SelectItem value="smb">{t("仅 SMB")}</SelectItem>
                <SelectItem value="ftp">{t("仅 FTP")}</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          <Field label={t("主机")}>
            <Input value={form.host} onChange={(event) => page.updateConnectionForm({ host: event.target.value })} placeholder="192.168.88.186" />
          </Field>
          <Field label={t("域（可选）")}>
            <Input value={form.domain} onChange={(event) => page.updateConnectionForm({ domain: event.target.value })} placeholder="WORKGROUP" />
          </Field>
          {page.uiContext?.platform === "windows" ? (
            <Field label={t("Windows SMB 登录方式")}>
              <Select value={form.windowsAuthMode} onValueChange={(value) => page.updateConnectionForm({ windowsAuthMode: value as typeof form.windowsAuthMode })}>
                <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="auto">{t("自动协商（推荐）")}</SelectItem>
                  <SelectItem value="plain">{t("仅用户名")}</SelectItem>
                  <SelectItem value="domain">{t("域\\用户名")}</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t("NAS 本地账户优先使用“主机\\用户名”，域账户使用“域\\用户名”。")}
              </p>
            </Field>
          ) : null}
          <Field label={t("用户名")}>
            <Input value={form.username} onChange={(event) => page.updateConnectionForm({ username: event.target.value })} autoComplete="username" />
          </Field>
          <Field label={t("密码")}>
            <Input type="text" value={form.password} onChange={(event) => page.updateConnectionForm({ password: event.target.value })} autoComplete="new-password" />
          </Field>
          <Field label={t("SMB 端口")}>
            <Input type="number" min={1} max={65535} value={form.smbPort} onChange={(event) => page.updateConnectionForm({ smbPort: event.target.value })} />
          </Field>
          <Field label={t("FTP 端口")}>
            <Input type="number" min={1} max={65535} value={form.ftpPort} onChange={(event) => page.updateConnectionForm({ ftpPort: event.target.value })} />
          </Field>
          <Field label="FTP TLS">
            <Select value={form.tlsMode} onValueChange={(value) => page.updateConnectionForm({ tlsMode: value as typeof form.tlsMode })}>
              <SelectTrigger className="w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="none">{t("关闭")}</SelectItem>
                <SelectItem value="explicit">{t("显式 TLS")}</SelectItem>
                <SelectItem value="implicit">{t("隐式 TLS")}</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          <label className="flex items-center justify-between gap-3 border p-3">
            <span className="text-sm">{t("跳过 FTP 证书校验")}</span>
            <Switch checked={form.noCheckCertificate} onCheckedChange={(checked) => page.updateConnectionForm({ noCheckCertificate: checked })} />
          </label>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => page.setConnectionDialogOpen(false)}>{t("取消")}</Button>
          <Button onClick={page.saveConnection} disabled={page.busyId === "save-connection"}>{t("保存连接")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <div className="space-y-2"><Label>{label}</Label>{children}</div>;
}
