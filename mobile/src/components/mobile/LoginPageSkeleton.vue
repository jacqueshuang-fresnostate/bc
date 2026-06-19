<template>
  <section class="login-page-skeleton" role="status" aria-label="正在加载登录页面" aria-busy="true">
    <div class="login-page-skeleton__brand">
      <span class="login-page-skeleton__logo"></span>
      <span class="login-page-skeleton__title"></span>
      <span class="login-page-skeleton__slogan"></span>
    </div>

    <div class="login-page-skeleton__card">
      <div class="login-page-skeleton__field">
        <span class="login-page-skeleton__label"></span>
        <span class="login-page-skeleton__input"></span>
      </div>
      <div class="login-page-skeleton__field">
        <span class="login-page-skeleton__label login-page-skeleton__label--short"></span>
        <span class="login-page-skeleton__input"></span>
      </div>
      <span class="login-page-skeleton__button"></span>
      <div class="login-page-skeleton__separator">
        <span></span>
        <em></em>
        <span></span>
      </div>
      <span class="login-page-skeleton__switch"></span>
    </div>
  </section>
</template>

<style scoped>
.login-page-skeleton {
  width: 100%;
  max-width: 24rem;
}

.login-page-skeleton__brand {
  display: flex;
  align-items: center;
  flex-direction: column;
  margin-bottom: 1rem;
}

.login-page-skeleton__card {
  display: grid;
  gap: 0.875rem;
  overflow: hidden;
  padding: 1.25rem;
  border: 1px solid rgba(140, 10, 21, 0.06);
  border-radius: 1.5rem;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 18px 48px rgba(140, 10, 21, 0.08);
}

.login-page-skeleton__field {
  display: grid;
  gap: 0.375rem;
}

.login-page-skeleton__logo,
.login-page-skeleton__title,
.login-page-skeleton__slogan,
.login-page-skeleton__label,
.login-page-skeleton__input,
.login-page-skeleton__button,
.login-page-skeleton__switch,
.login-page-skeleton__separator span,
.login-page-skeleton__separator em {
  position: relative;
  display: block;
  overflow: hidden;
  background: #efe3df;
}

.login-page-skeleton__logo::after,
.login-page-skeleton__title::after,
.login-page-skeleton__slogan::after,
.login-page-skeleton__label::after,
.login-page-skeleton__input::after,
.login-page-skeleton__button::after,
.login-page-skeleton__switch::after,
.login-page-skeleton__separator span::after,
.login-page-skeleton__separator em::after {
  content: '';
  position: absolute;
  inset: 0;
  transform: translateX(-100%);
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.64), transparent);
  animation: login-page-skeleton-shimmer 1.35s ease-in-out infinite;
}

.login-page-skeleton__logo {
  width: 4.5rem;
  height: 4.5rem;
  border-radius: 999px;
  box-shadow: 0 18px 44px rgba(140, 10, 21, 0.1);
}

.login-page-skeleton__title {
  width: 8.5rem;
  height: 1.8rem;
  margin-top: 0.625rem;
  border-radius: 999px;
}

.login-page-skeleton__slogan {
  width: 11rem;
  height: 0.8rem;
  margin-top: 0.375rem;
  border-radius: 999px;
}

.login-page-skeleton__label {
  width: 5.6rem;
  height: 0.75rem;
  margin-left: 0.25rem;
  border-radius: 999px;
}

.login-page-skeleton__label--short {
  width: 3rem;
}

.login-page-skeleton__input,
.login-page-skeleton__button {
  min-height: 3rem;
  border-radius: 1rem;
}

.login-page-skeleton__button {
  background: rgba(140, 10, 21, 0.16);
}

.login-page-skeleton__separator {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  min-height: 1rem;
}

.login-page-skeleton__separator span {
  height: 1px;
  flex: 1;
  border-radius: 999px;
}

.login-page-skeleton__separator em {
  width: 1.9rem;
  height: 0.55rem;
  flex: 0 0 auto;
  border-radius: 999px;
}

.login-page-skeleton__switch {
  width: 8.8rem;
  height: 0.9rem;
  margin: 0 auto;
  border-radius: 999px;
}

@keyframes login-page-skeleton-shimmer {
  100% {
    transform: translateX(100%);
  }
}

@media (max-height: 720px) {
  .login-page-skeleton__brand {
    margin-bottom: 0.75rem;
  }

  .login-page-skeleton__logo {
    width: 3.75rem;
    height: 3.75rem;
  }

  .login-page-skeleton__title {
    height: 1.55rem;
    margin-top: 0.5rem;
  }

  .login-page-skeleton__slogan {
    margin-top: 0.25rem;
  }

  .login-page-skeleton__card {
    gap: 0.625rem;
    padding: 1rem;
    border-radius: 1.25rem;
  }

  .login-page-skeleton__input,
  .login-page-skeleton__button {
    min-height: 2.625rem;
    border-radius: 0.875rem;
  }
}
</style>
