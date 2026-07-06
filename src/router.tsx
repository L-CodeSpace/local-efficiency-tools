import { lazy, Suspense } from "react";
import { createBrowserRouter, Navigate } from "react-router-dom";
import { ErrorBoundary } from "@/components/app/ErrorBoundary";
import { PageSkeleton } from "@/components/app/PageSkeleton";
import { RootLayout } from "@/components/app/RootLayout";

const ImageCompressorPage = lazy(() => import("@/pages/image-compressor"));
const VideoCompressorPage = lazy(() => import("@/pages/video-compressor"));
const DnsManagerPage = lazy(() => import("@/pages/dns-manager"));
const FileManagerPage = lazy(() => import("@/pages/file-manager"));
const BatchRenamePage = lazy(() => import("@/pages/batch-rename"));
const RemoteMountsPage = lazy(() => import("@/pages/remote-mounts"));
const SettingsPage = lazy(() => import("@/pages/settings"));
const SystemLogsPage = lazy(() => import("@/pages/system-logs"));

function lazyPage(element: React.ReactNode) {
  return <Suspense fallback={<PageSkeleton />}>{element}</Suspense>;
}

export const router = createBrowserRouter([
  {
    path: "/",
    element: <RootLayout />,
    errorElement: <ErrorBoundary />,
    children: [
      { index: true, element: <Navigate to="/image-compressor" replace /> },
      { path: "image-compressor", element: lazyPage(<ImageCompressorPage />) },
      { path: "video-compressor", element: lazyPage(<VideoCompressorPage />) },
      { path: "dns-manager", element: lazyPage(<DnsManagerPage />) },
      { path: "file-manager", element: lazyPage(<FileManagerPage />) },
      { path: "batch-rename", element: lazyPage(<BatchRenamePage />) },
      { path: "remote-mounts", element: lazyPage(<RemoteMountsPage />) },
      { path: "system-logs", element: lazyPage(<SystemLogsPage />) },
      { path: "settings", element: lazyPage(<SettingsPage />) },
      { path: "*", element: <Navigate to="/image-compressor" replace /> },
    ],
  },
]);
