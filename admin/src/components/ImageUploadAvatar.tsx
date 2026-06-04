import {
  Avatar,
  Banner,
  Button,
  Spin,
  Toast,
  Upload,
} from '@douyinfe/semi-ui';
import { IconCamera } from '@douyinfe/semi-icons';
import {
  CheckCircle,
  Copy,
  ExternalLink,
  Upload as UploadIcon,
  X,
} from 'lucide-react';
import { useEffect, useState, type CSSProperties } from 'react';
import { uploadImageBedFile } from '../api/client';

interface ImageUploadAvatarProps {
  clearLabel?: string;
  description?: string;
  disabled?: boolean;
  errorTitle?: string;
  failureMessage?: string;
  imageUrl?: string;
  missingConfigLabels?: string[];
  onClear?: () => void;
  onUploaded?: (url: string, response: unknown, file: File) => void;
  requireImageUrl?: boolean;
  showResultPanel?: boolean;
  successMessage?: string;
  title?: string;
  uploadFieldName?: string;
  uploadingText?: string;
  previewShape?: 'avatar' | 'banner';
  variant?: 'panel' | 'uploadAdd';
  warningTitle?: string;
}

export function ImageUploadAvatar({
  clearLabel = '清空图片',
  description,
  disabled = false,
  errorTitle = '图片上传失败',
  failureMessage = '上传失败',
  imageUrl = '',
  missingConfigLabels = [],
  onClear,
  onUploaded,
  requireImageUrl = false,
  showResultPanel = true,
  successMessage = '图片上传成功',
  title = '点击图片区域上传图片',
  uploadFieldName = 'file',
  uploadingText = '正在上传图片...',
  previewShape = 'avatar',
  variant = 'panel',
  warningTitle = '图床配置不完整',
}: ImageUploadAvatarProps) {
  const [copied, setCopied] = useState(false);
  const [file, setFile] = useState<File | null>(null);
  const [uploadError, setUploadError] = useState<string | null>(null);
  const [uploadResult, setUploadResult] = useState<unknown>(null);
  const [uploading, setUploading] = useState(false);
  const previewUrl = useObjectUrl(file);
  const resultUrl = extractImageUrlFromUploadResult(uploadResult);
  const avatarUrl = resultUrl || imageUrl || previewUrl;
  const resultText = uploadResult ? JSON.stringify(uploadResult, null, 2) : '';

  const uploadFile = async (nextFile: File) => {
    setCopied(false);
    setFile(nextFile);
    setUploadError(null);
    setUploadResult(null);
    setUploading(true);

    try {
      const response = await uploadImageBedFile(nextFile, uploadFieldName || 'file');
      const uploadedUrl = extractImageUrlFromUploadResult(response);
      if (requireImageUrl && !uploadedUrl) {
        throw new Error('图床返回未提供可用图片链接');
      }

      setUploadResult(response);
      onUploaded?.(uploadedUrl, response, nextFile);
      Toast.success(successMessage);
      return response;
    } catch (error: unknown) {
      setUploadError(error instanceof Error ? error.message : failureMessage);
      throw error;
    } finally {
      setUploading(false);
    }
  };

  const copyResultUrl = async () => {
    if (!resultUrl) {
      return;
    }

    await navigator.clipboard.writeText(resultUrl);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  };

  if (variant === 'uploadAdd') {
    return (
      <Upload
        accept="image/*"
        action="/api/admin/image-bed/upload"
        className="lottery-logo-upload"
        disabled={disabled || uploading || missingConfigLabels.length > 0}
        listType="picture"
        picHeight={96}
        picWidth={96}
        showUploadList={false}
        beforeUpload={() => {
          if (missingConfigLabels.length > 0) {
            Toast.warning('请先补全图床配置');
            return false;
          }
          return true;
        }}
        customRequest={(request) => {
          void uploadFile(request.fileInstance)
            .then((response) => request.onSuccess(response))
            .catch(() => request.onError({ status: 500 }));
        }}
        onError={() => Toast.error(failureMessage)}
      >
        {uploading ? (
          <Spin size="small" />
        ) : avatarUrl ? (
          <img
            alt="彩种 Logo"
            className="h-full w-full object-contain"
            src={avatarUrl}
          />
        ) : (
          <UploadIcon size={28} />
        )}
      </Upload>
    );
  }

  return (
    <div className="space-y-3">
      {missingConfigLabels.length > 0 ? (
        <Banner
          description={`请先保存：${missingConfigLabels.join('、')}`}
          title={warningTitle}
          type="warning"
        />
      ) : null}

      <div className="grid gap-3 rounded border border-slate-200 bg-white p-4">
        {previewShape === 'banner' ? (
          <div className="space-y-3">
            <div className="min-w-0">
              <p className="text-sm font-medium text-ink">{title}</p>
              <p className="mt-1 text-xs text-slate-500">{description}</p>
              {file ? (
                <p className="mt-2 truncate text-xs text-slate-500">
                  当前文件：{file.name}，{formatFileSize(file.size)}
                </p>
              ) : null}
            </div>

            <Upload
              accept="image/*"
              action="/api/admin/image-bed/upload"
              className="block w-full"
              disabled={disabled || uploading || missingConfigLabels.length > 0}
              showUploadList={false}
              beforeUpload={() => {
                if (missingConfigLabels.length > 0) {
                  Toast.warning('请先补全图床配置');
                  return false;
                }
                return true;
              }}
              customRequest={(request) => {
                void uploadFile(request.fileInstance)
                  .then((response) => request.onSuccess(response))
                  .catch(() => request.onError({ status: 500 }));
              }}
              onError={() => Toast.error(failureMessage)}
            >
              <div className="group relative flex aspect-[16/7] min-h-[150px] w-full cursor-pointer items-center justify-center overflow-hidden rounded-md border border-dashed border-slate-300 bg-slate-50 text-slate-500">
                {avatarUrl ? (
                  <img
                    alt={title}
                    className="h-full w-full object-cover"
                    src={avatarUrl}
                  />
                ) : (
                  <div className="flex flex-col items-center gap-2 text-sm">
                    <UploadIcon size={34} />
                    <span>上传长方形广告图</span>
                  </div>
                )}
                <div className="absolute inset-0 hidden items-center justify-center bg-slate-900/55 text-white group-hover:flex">
                  <IconCamera />
                </div>
              </div>
            </Upload>
          </div>
        ) : (
          <div className="flex items-center gap-4">
            <Upload
              accept="image/*"
              action="/api/admin/image-bed/upload"
              className="avatar-upload"
              disabled={disabled || uploading || missingConfigLabels.length > 0}
              showUploadList={false}
              beforeUpload={() => {
                if (missingConfigLabels.length > 0) {
                  Toast.warning('请先补全图床配置');
                  return false;
                }
                return true;
              }}
              customRequest={(request) => {
                void uploadFile(request.fileInstance)
                  .then((response) => request.onSuccess(response))
                  .catch(() => request.onError({ status: 500 }));
              }}
              onError={() => Toast.error(failureMessage)}
            >
              <Avatar
                hoverMask={avatarHoverMask}
                shape="square"
                size="extra-large"
                src={avatarUrl || undefined}
                style={{
                  backgroundColor: 'var(--semi-color-fill-0)',
                  color: 'var(--semi-color-text-2)',
                  margin: 4,
                }}
              >
                <UploadIcon size={34} />
              </Avatar>
            </Upload>

            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-ink">{title}</p>
              <p className="mt-1 text-xs text-slate-500">{description}</p>
              {file ? (
                <p className="mt-2 truncate text-xs text-slate-500">
                  当前文件：{file.name}，{formatFileSize(file.size)}
                </p>
              ) : null}
            </div>
          </div>
        )}

        {uploading ? (
          <div className="flex items-center gap-2 rounded border border-blue-100 bg-blue-50 px-3 py-2 text-sm text-blue-700">
            <Spin size="small" />
            {uploadingText}
          </div>
        ) : null}

        {onClear && imageUrl ? (
          <Button className="w-fit" icon={<X size={16} />} onClick={onClear}>
            {clearLabel}
          </Button>
        ) : null}
      </div>

      {uploadError ? (
        <Banner description={uploadError} title={errorTitle} type="danger" />
      ) : null}

      {showResultPanel && uploadResult ? (
        <div className="space-y-3 rounded border border-emerald-100 bg-emerald-50 p-3">
          <div className="flex items-center gap-2 text-sm font-medium text-emerald-700">
            <CheckCircle size={16} />
            上传测试成功
          </div>
          {resultUrl ? (
            <div className="space-y-2 rounded border border-emerald-100 bg-white p-3">
              <p className="text-xs text-slate-500">图片链接</p>
              <a
                className="block break-all text-sm text-teal-700 hover:text-teal-800"
                href={resultUrl}
                rel="noreferrer"
                target="_blank"
              >
                {resultUrl}
              </a>
              <div className="flex flex-wrap gap-2">
                <Button
                  icon={<Copy size={14} />}
                  size="small"
                  onClick={() => {
                    void copyResultUrl();
                  }}
                >
                  {copied ? '已复制' : '复制链接'}
                </Button>
                <Button
                  icon={<ExternalLink size={14} />}
                  size="small"
                  onClick={() => window.open(resultUrl, '_blank', 'noreferrer')}
                >
                  打开图片
                </Button>
              </div>
              <img
                alt="图片上传结果预览"
                className="max-h-40 rounded border border-slate-200 object-contain"
                src={resultUrl}
              />
            </div>
          ) : (
            <Banner
              description="当前返回内容未识别出可直接展示的图片链接，请检查返回链接字段配置。"
              title="未提取到图片链接"
              type="warning"
            />
          )}
          <details className="rounded border border-emerald-100 bg-white p-3">
            <summary className="cursor-pointer text-sm font-medium text-slate-700">
              查看原始返回
            </summary>
            <pre className="mt-3 max-h-48 overflow-auto rounded bg-slate-50 p-3 text-xs text-slate-700">
              {resultText}
            </pre>
          </details>
        </div>
      ) : null}
    </div>
  );
}

const avatarMaskStyle: CSSProperties = {
  alignItems: 'center',
  backgroundColor: 'var(--semi-color-overlay-bg)',
  color: 'var(--semi-color-white)',
  display: 'flex',
  height: '100%',
  justifyContent: 'center',
  width: '100%',
};

const avatarHoverMask = (
  <div style={avatarMaskStyle}>
    <IconCamera />
  </div>
);

function useObjectUrl(file: File | null) {
  const [objectUrl, setObjectUrl] = useState('');

  useEffect(() => {
    if (!file) {
      setObjectUrl('');
      return undefined;
    }

    const nextObjectUrl = URL.createObjectURL(file);
    setObjectUrl(nextObjectUrl);
    return () => URL.revokeObjectURL(nextObjectUrl);
  }, [file]);

  return objectUrl;
}

function formatFileSize(size: number) {
  if (size < 1024) {
    return `${size} B`;
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`;
  }
  return `${(size / 1024 / 1024).toFixed(1)} MB`;
}

function extractImageUrlFromUploadResult(result: unknown): string {
  if (typeof result === 'string') {
    return result.trim();
  }
  if (!result || typeof result !== 'object') {
    return '';
  }

  const candidates = [
    ['url'],
    ['download'],
    ['link'],
    ['links', 'download'],
    ['links', 'share'],
    ['file', 'url'],
  ];
  for (const path of candidates) {
    const value = readObjectPath(result, path);
    if (typeof value === 'string' && value.trim()) {
      return value.trim();
    }
  }
  return '';
}

function readObjectPath(value: unknown, path: string[]): unknown {
  let current = value;
  for (const key of path) {
    if (!current || typeof current !== 'object' || !(key in current)) {
      return undefined;
    }
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}
