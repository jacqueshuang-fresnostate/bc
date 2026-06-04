export type AdvertisementPlacement = 'mobileCarousel';
export type AdvertisementStatus = 'enabled' | 'disabled';

export interface AdvertisementSummary {
  id: string;
  title: string;
  imageUrl: string;
  linkUrl: string | null;
  placement: AdvertisementPlacement;
  status: AdvertisementStatus;
  sortOrder: number;
  startAt: string | null;
  endAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SaveAdvertisementRequest {
  id?: string;
  title: string;
  imageUrl: string;
  linkUrl?: string | null;
  placement: AdvertisementPlacement;
  status: AdvertisementStatus;
  sortOrder: number;
  startAt?: string | null;
  endAt?: string | null;
}
