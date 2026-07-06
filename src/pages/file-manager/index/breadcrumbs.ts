/*
 * 核心职责：构建文件管理路径面包屑。
 * 业务痛点：路径拆分规则与页面渲染混杂会增加文件管理入口体量。
 * 能力边界：只处理路径到面包屑列表的纯转换。
 */

export function buildCrumbs(path: string) {
  if (!path) return [];
  const sep = path.includes("\\") ? "\\" : "/";
  const parts = path.split(/[/\\]/).filter(Boolean);
  const crumbs: { label: string; path: string }[] = [];
  let acc = path.includes("\\") ? "" : "/";
  for (let index = 0; index < parts.length; index += 1) {
    acc = index === 0 && path.includes("\\") ? `${parts[0]}${sep}` : `${acc}${parts[index]}${sep}`;
    crumbs.push({ label: parts[index], path: acc.replace(/[/\\]$/, "") || sep });
  }
  return crumbs;
}
