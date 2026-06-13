import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/login', name: 'Login', component: () => import('../views/LoginView.vue'), meta: { guestOnly: true } },
    { path: '/forgot-password', name: 'ForgotPassword', component: () => import('../views/ForgotPasswordView.vue'), meta: { guestOnly: true } },
    {
      path: '/',
      component: () => import('../views/LayoutView.vue'),
      meta: { requiresAuth: true },
      children: [
        { path: '', name: 'Home', component: () => import('../views/HomeView.vue') },
        { path: 'lotteries', name: 'AllLotteries', component: () => import('../views/AllLotteryView.vue') },
        { path: 'bet/:code', name: 'Bet', component: () => import('../views/BetView.vue') },
        { path: 'history', name: 'History', component: () => import('../views/HistoryView.vue') },
        { path: 'group-buy', name: 'GroupBuy', component: () => import('../views/GroupBuyView.vue') },
        { path: 'orders', name: 'Orders', component: () => import('../views/HistoryView.vue') },
        { path: 'deposit', name: 'Deposit', component: () => import('../views/DepositView.vue') },
        { path: 'support', name: 'Support', component: () => import('../views/SupportView.vue') },
        { path: 'chat-hall', name: 'ChatHall', component: () => import('../views/ChatHallView.vue') },
        { path: 'withdraw', name: 'Withdraw', component: () => import('../views/WithdrawView.vue') },
        { path: 'withdrawal-methods', name: 'WithdrawalMethods', component: () => import('../views/WithdrawalMethodsView.vue') },
        { path: 'ledger', name: 'AccountLedger', component: () => import('../views/AccountLedgerView.vue') },
        { path: 'agent-center', name: 'AgentCenter', component: () => import('../views/InvitationCenterView.vue') },
        { path: 'invitation-center', name: 'InvitationCenter', component: () => import('../views/InvitationCenterView.vue') },
        { path: 'security-center', name: 'SecurityCenter', component: () => import('../views/SecurityCenterView.vue') },
        { path: 'me', name: 'Profile', component: () => import('../views/ProfileView.vue') },
      ],
    },
  ],
})

router.beforeEach((to) => {
  const auth = useAuthStore()
  const isLoggedIn = !!auth.accessToken

  if (to.meta.requiresAuth && !isLoggedIn) {
    return {
      path: '/login',
      query: to.fullPath && to.fullPath !== '/' ? { redirect: to.fullPath } : undefined,
      replace: true,
    }
  }

  if (to.meta.guestOnly && isLoggedIn) {
    const redirect = typeof to.query.redirect === 'string' && to.query.redirect ? to.query.redirect : '/'
    return { path: redirect, replace: true }
  }

  return true
})

export default router
