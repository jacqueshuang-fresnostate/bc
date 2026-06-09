export type SupportConversationStatus = 'open' | 'pending' | 'resolved' | 'closed';
export type SupportPriority = 'normal' | 'urgent';
export type SupportMessageAuthor = 'user' | 'admin' | 'system';
export type SupportMessageType = 'text' | 'image';

export interface SupportMessage {
  id: string;
  author: SupportMessageAuthor;
  authorId: string;
  authorName: string;
  messageType: SupportMessageType;
  content: string;
  imageUrl: string | null;
  createdAt: string;
}

export interface SupportConversation {
  id: string;
  userId: string;
  username: string;
  subject: string;
  status: SupportConversationStatus;
  priority: SupportPriority;
  assignedAdminId: string | null;
  assignedAdminName: string | null;
  unreadCount: number;
  createdAt: string;
  updatedAt: string;
  messages: SupportMessage[];
}

export interface CreateSupportConversationRequest {
  id: string;
  userId: string;
  subject: string;
  priority: SupportPriority;
  content: string;
}

export interface UpdateSupportConversationRequest {
  status: SupportConversationStatus;
  priority: SupportPriority;
  assignedAdminId: string | null;
}

export interface SupportReplyRequest {
  adminId: string;
  content: string;
  imageUrl?: string | null;
  messageType?: SupportMessageType;
}
