//! 列表分页工具，统一后端仓储和路由之间的分页口径。

use crate::{
    domain::finance::FinancePage,
    error::{ApiError, ApiResult},
};

#[derive(Clone, Copy, Debug, Default)]
/// 仓储列表查询分页请求；未传分页参数时保持旧接口的全量返回语义。
pub struct PageRequest {
    page: Option<usize>,
    page_size: Option<usize>,
}

impl PageRequest {
    /// 从接口查询参数创建分页请求。
    pub fn new(page: Option<usize>, page_size: Option<usize>) -> Self {
        Self { page, page_size }
    }

    /// 判断当前请求是否显式分页。
    pub fn is_paginated(self) -> bool {
        self.page.is_some() || self.page_size.is_some()
    }

    /// 根据总数计算最终页码、每页条数和 SQL `LIMIT/OFFSET`。
    pub fn resolve(self, total_count: usize) -> ResolvedPage {
        if !self.is_paginated() {
            let page_size = total_count.max(1);
            let total_pages = if total_count == 0 { 0 } else { 1 };
            return ResolvedPage {
                page: 1,
                page_size,
                offset: 0,
                limit: page_size,
                total_count,
                total_pages,
            };
        }

        let page_size = self.page_size.unwrap_or(20).max(1);
        let max_page = if total_count == 0 {
            1
        } else {
            total_count.div_ceil(page_size)
        };
        let page = self.page.unwrap_or(1).max(1).min(max_page);
        let offset = (page - 1).saturating_mul(page_size);
        let limit = page_size;
        let total_pages = if total_count == 0 {
            0
        } else {
            total_count.div_ceil(page_size)
        };

        ResolvedPage {
            page,
            page_size,
            offset,
            limit,
            total_count,
            total_pages,
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// 已按总数归一化后的分页参数，供 SQL 查询和响应结构共用。
pub struct ResolvedPage {
    /// 当前页码，从 1 开始。
    pub page: usize,
    /// 每页记录数量。
    pub page_size: usize,
    /// 偏移字段。
    pub offset: usize,
    /// limit字段。
    pub limit: usize,
    /// 符合条件的总记录数。
    pub total_count: usize,
    /// 总页数。
    pub total_pages: usize,
}

impl ResolvedPage {
    /// 转成 SQLx 可绑定的 `LIMIT` 数值。
    pub fn limit_i64(self) -> ApiResult<i64> {
        i64::try_from(self.limit).map_err(|_| ApiError::BadRequest("分页条数过大".to_string()))
    }

    /// 转成 SQLx 可绑定的 `OFFSET` 数值。
    pub fn offset_i64(self) -> ApiResult<i64> {
        i64::try_from(self.offset).map_err(|_| ApiError::BadRequest("分页偏移过大".to_string()))
    }
}

#[derive(Clone, Debug)]
/// 仓储分页查询结果，包含当前页数据和分页元信息。
pub struct ListPage<T> {
    /// 分页数据列表。
    pub items: Vec<T>,
    /// 当前页码，从 1 开始。
    pub page: usize,
    /// 每页记录数量。
    pub page_size: usize,
    /// 符合条件的总记录数。
    pub total_count: usize,
    /// 总页数。
    pub total_pages: usize,
}

impl<T> ListPage<T> {
    /// 从已经排序过滤好的完整列表中生成分页结果，供内存仓储和复杂派生列表复用。
    pub fn from_all(items: Vec<T>, request: PageRequest) -> Self {
        let total_count = items.len();
        let resolved = request.resolve(total_count);
        let items = items
            .into_iter()
            .skip(resolved.offset)
            .take(resolved.limit)
            .collect();

        Self::new(items, resolved)
    }

    /// 从 SQL 查询返回的当前页数据和分页参数创建结果。
    pub fn new(items: Vec<T>, resolved: ResolvedPage) -> Self {
        Self {
            items,
            page: resolved.page,
            page_size: resolved.page_size,
            total_count: resolved.total_count,
            total_pages: resolved.total_pages,
        }
    }

    /// 转为后台现有的分页响应结构，保持 API 契约不变。
    pub fn into_finance_page(self) -> FinancePage<T> {
        FinancePage {
            items: self.items,
            page: self.page,
            page_size: self.page_size,
            total_count: self.total_count,
            total_pages: self.total_pages,
        }
    }
}
