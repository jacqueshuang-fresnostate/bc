<template>
  <section class="bet-page-skeleton" role="status" aria-label="正在加载投注页" aria-busy="true">
    <div class="bet-page-skeleton__round">
      <div>
        <span class="bet-page-skeleton__line bet-page-skeleton__line--xs"></span>
        <span class="bet-page-skeleton__line bet-page-skeleton__line--lg"></span>
      </div>
      <div class="bet-page-skeleton__timer"></div>
    </div>

    <div class="bet-page-skeleton__card bet-page-skeleton__play">
      <div class="bet-page-skeleton__stack">
        <span class="bet-page-skeleton__line bet-page-skeleton__line--xs"></span>
        <span class="bet-page-skeleton__line bet-page-skeleton__line--xl"></span>
        <span class="bet-page-skeleton__line bet-page-skeleton__line--md"></span>
      </div>
      <span class="bet-page-skeleton__pill"></span>
    </div>

    <div class="bet-page-skeleton__card">
      <div class="bet-page-skeleton__section-head">
        <span class="bet-page-skeleton__line bet-page-skeleton__line--md"></span>
        <span class="bet-page-skeleton__pill bet-page-skeleton__pill--short"></span>
      </div>
      <div class="bet-page-skeleton__number-grid">
        <span v-for="index in 20" :key="index" class="bet-page-skeleton__ball"></span>
      </div>
    </div>

    <div class="bet-page-skeleton__card">
      <div class="bet-page-skeleton__section-head">
        <div class="bet-page-skeleton__stack">
          <span class="bet-page-skeleton__line bet-page-skeleton__line--sm"></span>
          <span class="bet-page-skeleton__line bet-page-skeleton__line--md"></span>
        </div>
        <div class="bet-page-skeleton__stepper">
          <span></span>
          <span></span>
          <span></span>
        </div>
      </div>
      <span class="bet-page-skeleton__slider"></span>
    </div>

    <div class="bet-page-skeleton__bottom" aria-hidden="true">
      <div class="bet-page-skeleton__bottom-inner">
        <div class="bet-page-skeleton__stack">
          <span class="bet-page-skeleton__line bet-page-skeleton__line--sm"></span>
          <span class="bet-page-skeleton__line bet-page-skeleton__line--md"></span>
        </div>
        <span class="bet-page-skeleton__submit"></span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.bet-page-skeleton {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding-bottom: 1.5rem;
}

.bet-page-skeleton__round,
.bet-page-skeleton__card {
  overflow: hidden;
  border: 1px solid rgba(140, 10, 21, 0.06);
  border-radius: 1.75rem;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 8px 28px rgba(140, 10, 21, 0.04);
}

.bet-page-skeleton__round {
  display: flex;
  min-height: 6.8rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 1rem;
}

.bet-page-skeleton__card {
  padding: 1rem;
}

.bet-page-skeleton__play {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.bet-page-skeleton__stack {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 0.55rem;
}

.bet-page-skeleton__section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.bet-page-skeleton__line,
.bet-page-skeleton__pill,
.bet-page-skeleton__timer,
.bet-page-skeleton__ball,
.bet-page-skeleton__slider,
.bet-page-skeleton__stepper span,
.bet-page-skeleton__submit {
  position: relative;
  overflow: hidden;
  background: #efe3df;
}

.bet-page-skeleton__line::after,
.bet-page-skeleton__pill::after,
.bet-page-skeleton__timer::after,
.bet-page-skeleton__ball::after,
.bet-page-skeleton__slider::after,
.bet-page-skeleton__stepper span::after,
.bet-page-skeleton__submit::after {
  content: '';
  position: absolute;
  inset: 0;
  transform: translateX(-100%);
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.62), transparent);
  animation: bet-page-skeleton-shimmer 1.35s ease-in-out infinite;
}

.bet-page-skeleton__line {
  display: block;
  height: 0.75rem;
  border-radius: 999px;
}

.bet-page-skeleton__line--xs {
  width: 4.6rem;
}

.bet-page-skeleton__line--sm {
  width: 5.8rem;
}

.bet-page-skeleton__line--md {
  width: 8rem;
}

.bet-page-skeleton__line--lg {
  width: 10rem;
  height: 1rem;
}

.bet-page-skeleton__line--xl {
  width: min(14rem, 62vw);
  height: 1.3rem;
}

.bet-page-skeleton__pill {
  display: block;
  width: 3.8rem;
  height: 1.75rem;
  flex: 0 0 auto;
  border-radius: 999px;
}

.bet-page-skeleton__pill--short {
  width: 3rem;
  height: 1.35rem;
}

.bet-page-skeleton__timer {
  width: 4.8rem;
  height: 3.7rem;
  flex: 0 0 auto;
  border-radius: 1.15rem;
}

.bet-page-skeleton__number-grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 0.7rem;
  margin-top: 1rem;
}

.bet-page-skeleton__ball {
  display: block;
  width: 100%;
  aspect-ratio: 1 / 1;
  border-radius: 999px;
}

.bet-page-skeleton__stepper {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 0.35rem;
  border-radius: 1.25rem;
  padding: 0.25rem;
  background: #fffdfc;
}

.bet-page-skeleton__stepper span {
  display: block;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 0.9rem;
}

.bet-page-skeleton__slider {
  display: block;
  height: 0.45rem;
  margin-top: 1.25rem;
  border-radius: 999px;
}

.bet-page-skeleton__bottom {
  position: fixed;
  right: 0;
  bottom: var(--mobile-viewport-bottom-inset);
  left: 0;
  z-index: 48;
  padding: 1.1rem 1.5rem 1.5rem;
  background: rgba(249, 249, 249, 0.94);
  box-shadow: 0 -8px 40px rgba(140, 10, 21, 0.06);
  backdrop-filter: blur(12px);
}

@supports (padding-bottom: max(1px, 2px)) {
  .bet-page-skeleton__bottom {
    padding-bottom: max(1.5rem, env(safe-area-inset-bottom, 0px));
  }
}

.bet-page-skeleton__bottom-inner {
  display: flex;
  max-width: 28rem;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin: 0 auto;
}

.bet-page-skeleton__submit {
  display: block;
  width: 8rem;
  height: 3.25rem;
  flex: 0 0 auto;
  border-radius: 1.15rem;
}

@keyframes bet-page-skeleton-shimmer {
  100% {
    transform: translateX(100%);
  }
}

@media (max-width: 374px) {
  .bet-page-skeleton__number-grid {
    gap: 0.55rem;
  }

  .bet-page-skeleton__timer {
    width: 4.2rem;
  }

  .bet-page-skeleton__submit {
    width: 7rem;
  }
}
</style>
