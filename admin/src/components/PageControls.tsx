import { Button, Select } from '@douyinfe/semi-ui';

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];

interface PageControlsProps {
  loading: boolean;
  page: number;
  pageSize: number;
  totalCount: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
}

export function PageControls({
  loading,
  page,
  pageSize,
  totalCount,
  totalPages,
  onPageChange,
  onPageSizeChange,
}: PageControlsProps) {
  // 后台运营列表共用的分页条，统一总数、每页条数和翻页入口的展示方式。
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500">
      <span>共 {totalCount} 条</span>
      <label className="flex items-center gap-1">
        每页
        <Select
          className="form-input min-w-[86px]"
          value={pageSize}
          onChange={(value) => onPageSizeChange(Number(value ?? 10))}
        >
          {PAGE_SIZE_OPTIONS.map((size) => (
            <Select.Option key={size} value={size}>
              {size}
            </Select.Option>
          ))}
        </Select>
        条
      </label>
      <Button
        disabled={loading || page <= 1 || totalPages === 0}
        size="small"
        onClick={() => onPageChange(page - 1)}
      >
        上一页
      </Button>
      <span>
        第 {totalPages === 0 ? 0 : page} / {totalPages} 页
      </span>
      <Button
        disabled={loading || page >= totalPages || totalPages === 0}
        size="small"
        onClick={() => onPageChange(page + 1)}
      >
        下一页
      </Button>
    </div>
  );
}
