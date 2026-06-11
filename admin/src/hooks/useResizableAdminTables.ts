import { useEffect } from 'react';

const TABLE_SELECTOR = 'table';
const MIN_COLUMN_WIDTH = 72;
const RESIZE_EDGE_WIDTH = 10;

interface TableResizeState {
  columnIndex: number;
  startX: number;
  startTableWidth: number;
  startWidth: number;
  table: HTMLTableElement;
  widths: number[];
}

interface RegisteredTable {
  cleanup: () => void;
  headers: HTMLTableCellElement[];
}

/// 给管理后台所有原生表格增加列宽拖拽能力。
export function useResizableAdminTables() {
  useEffect(() => {
    const registeredTables = new WeakMap<HTMLTableElement, RegisteredTable>();
    const cleanupCallbacks = new Set<() => void>();
    let resizeState: TableResizeState | null = null;
    let scheduled = false;

    const processTables = () => {
      scheduled = false;
      document
        .querySelectorAll<HTMLTableElement>(TABLE_SELECTOR)
        .forEach((table) => registerTable(table, registeredTables, cleanupCallbacks));
    };

    const scheduleProcessTables = () => {
      if (scheduled) {
        return;
      }
      scheduled = true;
      window.requestAnimationFrame(processTables);
    };

    const stopResize = () => {
      resizeState = null;
      document.body.classList.remove('admin-table-column-resizing');
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', stopResize);
      window.removeEventListener('pointercancel', stopResize);
    };

    const handlePointerMove = (event: PointerEvent) => {
      if (!resizeState) {
        return;
      }

      const nextWidth = Math.max(
        MIN_COLUMN_WIDTH,
        resizeState.startWidth + event.clientX - resizeState.startX,
      );
      const widths = [...resizeState.widths];
      widths[resizeState.columnIndex] = nextWidth;
      const tableWidth =
        resizeState.startTableWidth + nextWidth - resizeState.startWidth;

      applyColumnWidths(resizeState.table, widths, tableWidth);
      event.preventDefault();
    };

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }
      const header = target.closest<HTMLTableCellElement>('th[data-admin-resizable-column]');
      if (!header || event.button !== 0) {
        return;
      }
      const table = header.closest('table');
      if (!(table instanceof HTMLTableElement)) {
        return;
      }
      const record = registeredTables.get(table);
      if (!record) {
        return;
      }
      const rect = header.getBoundingClientRect();
      if (rect.right - event.clientX > RESIZE_EDGE_WIDTH) {
        return;
      }

      const columnIndex = record.headers.indexOf(header);
      if (columnIndex < 0) {
        return;
      }

      const widths = measuredColumnWidths(record.headers);
      const startTableWidth = widths.reduce((total, width) => total + width, 0);
      resizeState = {
        columnIndex,
        startX: event.clientX,
        startTableWidth,
        startWidth: widths[columnIndex],
        table,
        widths,
      };
      applyColumnWidths(table, widths, startTableWidth);
      document.body.classList.add('admin-table-column-resizing');
      window.addEventListener('pointermove', handlePointerMove);
      window.addEventListener('pointerup', stopResize);
      window.addEventListener('pointercancel', stopResize);
      event.preventDefault();
    };

    const observer = new MutationObserver(scheduleProcessTables);
    observer.observe(document.body, { childList: true, subtree: true });
    processTables();

    return () => {
      observer.disconnect();
      stopResize();
      cleanupCallbacks.forEach((cleanup) => cleanup());
      cleanupCallbacks.clear();
    };

    function registerTable(
      table: HTMLTableElement,
      records: WeakMap<HTMLTableElement, RegisteredTable>,
      cleanups: Set<() => void>,
    ) {
      const headers = firstHeaderRowCells(table);
      if (headers.length === 0) {
        return;
      }

      const existing = records.get(table);
      if (existing && sameHeaders(existing.headers, headers)) {
        headers.forEach(markResizableHeader);
        return;
      }
      existing?.cleanup();
      if (existing) {
        cleanups.delete(existing.cleanup);
      }

      table.classList.add('admin-resizable-table');
      headers.forEach(markResizableHeader);
      table.addEventListener('pointerdown', handlePointerDown);

      const cleanup = () => {
        table.removeEventListener('pointerdown', handlePointerDown);
        table.classList.remove('admin-resizable-table');
        headers.forEach((header) => {
          delete header.dataset.adminResizableColumn;
        });
      };
      records.set(table, { cleanup, headers });
      cleanups.add(cleanup);
    }
  }, []);
}

/// 获取首行表头单元格，当前后台表格均使用单层表头。
function firstHeaderRowCells(table: HTMLTableElement) {
  const headerRow = table.tHead?.rows.item(0);
  if (!headerRow) {
    return [];
  }

  return Array.from(headerRow.cells).filter(
    (cell): cell is HTMLTableCellElement =>
      cell instanceof HTMLTableCellElement && cell.colSpan === 1,
  );
}

/// 标记表头列可拖拽，实际手柄由全局 CSS 伪元素渲染。
function markResizableHeader(header: HTMLTableCellElement) {
  header.dataset.adminResizableColumn = 'true';
}

/// 判断已注册表头是否仍是同一组 DOM 节点。
function sameHeaders(current: HTMLTableCellElement[], next: HTMLTableCellElement[]) {
  return current.length === next.length && current.every((cell, index) => cell === next[index]);
}

/// 读取当前渲染列宽，作为拖拽起点。
function measuredColumnWidths(headers: HTMLTableCellElement[]) {
  return headers.map((header) =>
    Math.max(MIN_COLUMN_WIDTH, Math.round(header.getBoundingClientRect().width)),
  );
}

/// 将列宽应用到每一行同列单元格，保证整列一起变化。
function applyColumnWidths(table: HTMLTableElement, widths: number[], tableWidth: number) {
  table.style.tableLayout = 'fixed';
  table.style.width = `${Math.round(tableWidth)}px`;
  table.style.minWidth = `${Math.round(tableWidth)}px`;

  Array.from(table.rows).forEach((row) => {
    let columnIndex = 0;
    Array.from(row.cells).forEach((cell) => {
      const span = Math.max(1, cell.colSpan);
      if (span === 1 && columnIndex < widths.length) {
        const width = `${Math.round(widths[columnIndex])}px`;
        cell.style.width = width;
        cell.style.minWidth = width;
        cell.style.maxWidth = width;
      }
      columnIndex += span;
    });
  });
}
