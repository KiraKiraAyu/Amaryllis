<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue"
import Button from "primevue/button"
import BaseInput from "@/components/universal/BaseInput.vue"
import type { AuthMode } from "@/types/auth-ui"

defineProps<{
  loading: boolean
  emailError: string
  passwordError: string
}>()

const mode = defineModel<AuthMode>("mode", { required: true })
const email = defineModel<string>("email", { required: true })
const password = defineModel<string>("password", { required: true })

const emit = defineEmits<{
  submit: []
  validate: [field: "email" | "password"]
}>()

function toggleMode() {
  mode.value = mode.value === "Register" ? "Login" : "Register"
}

function submitAs(nextMode: AuthMode) {
  mode.value = nextMode
  emit("submit")
}

const isDark = ref(false)
let observer: MutationObserver | null = null

function updateTheme() {
  isDark.value = document.documentElement.classList.contains("dark")
}

onMounted(() => {
  updateTheme()
  observer = new MutationObserver(updateTheme)
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] })
})

onUnmounted(() => {
  if (observer) {
    observer.disconnect()
  }
})

const backgroundStyle = computed(() => {
  if (isDark.value) {
    return {
      backgroundColor: "oklch(0.145 0.018 285)",
      backgroundImage: `
        radial-gradient(circle at top left, oklch(0.32 0.11 301), transparent 34%),
        radial-gradient(circle at bottom right, oklch(0.24 0.11 225), transparent 32%)
      `
    }
  } else {
    return {
      backgroundColor: "var(--p-surface-50)",
      backgroundImage: `
        radial-gradient(circle at top left, var(--color-reisa-pink-100), transparent 40%),
        radial-gradient(circle at bottom right, var(--color-reisa-lilac-100), transparent 40%)
      `
    }
  }
})
</script>

<template>
  <div
    class="min-h-screen flex items-center justify-center p-4 transition-colors duration-300"
    :style="backgroundStyle"
  >
    <div
      class="relative w-full max-w-4xl overflow-hidden rounded-4xl bg-surface-0 dark:bg-[oklch(0.19_0.018_285)] shadow-2xl shadow-surface-300/30 dark:shadow-black/50 min-h-155 transition-all duration-300"
    >
      <div
        class="absolute left-0 top-0 hidden h-full w-1/2 transition-all duration-700 ease-in-out md:block"
        :class="mode === 'Register' ? 'translate-x-full opacity-0 pointer-events-none z-10' : 'opacity-100 z-20'"
      >
        <form
          class="flex h-full flex-col items-center justify-center bg-surface-0 dark:bg-[oklch(0.18_0.018_285)] px-12 text-center text-surface-900 dark:text-white transition-colors duration-300"
          @submit.prevent="submitAs('Login')"
        >
          <div
            class="mb-5 inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-linear-to-br from-reisa-pink-500 to-reisa-lilac-500 shadow-lg shadow-reisa-pink-500/20"
          >
            <span class="text-2xl font-black text-white">A</span>
          </div>
          <h1 class="mb-2 text-3xl font-black text-surface-900 dark:text-white">Welcome back</h1>
          <p class="mb-6 text-sm text-surface-500 dark:text-white/55">Sign in to QuantAura</p>

          <BaseInput
            v-model="email"
            id="login-email"
            label="Email"
            type="email"
            autocomplete="email"
            required
            class="w-full"
            :error="emailError"
            @blur="emit('validate', 'email')"
          />
          <BaseInput
            v-model="password"
            id="login-password"
            label="Password"
            type="password"
            autocomplete="current-password"
            required
            class="w-full"
            :error="passwordError"
            @blur="emit('validate', 'password')"
          />

          <a
            href="#"
            class="my-4 border-b border-transparent text-sm text-surface-500 hover:border-surface-700 hover:text-surface-900 dark:text-white/45 dark:hover:border-white/45 dark:hover:text-white transition-colors"
          >
            Forgot password?
          </a>
          <Button
            type="submit"
            class="mt-2 min-w-36 justify-center font-semibold uppercase tracking-wider rounded-full py-2 bg-linear-to-br from-reisa-pink-500 to-reisa-lilac-500 hover:from-reisa-pink-400 hover:to-reisa-lilac-400 border-none text-white transition-all duration-200"
            :disabled="loading"
            :loading="loading"
          >
            Log in
          </Button>
        </form>
      </div>

      <div
        class="absolute left-0 top-0 hidden h-full w-1/2 transition-all duration-700 ease-in-out md:block"
        :class="
          mode === 'Register'
            ? 'translate-x-full opacity-100 z-30'
            : 'opacity-0 z-10 pointer-events-none'
        "
      >
        <form
          class="flex h-full flex-col items-center justify-center bg-surface-0 dark:bg-[oklch(0.18_0.018_285)] px-12 text-center text-surface-900 dark:text-white transition-colors duration-300"
          @submit.prevent="submitAs('Register')"
        >
          <div
            class="mb-5 inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-linear-to-br from-reisa-pink-500 to-reisa-lilac-500 shadow-lg shadow-reisa-pink-500/20"
          >
            <span class="text-2xl font-black text-white">A</span>
          </div>
          <h1 class="mb-2 text-3xl font-black text-surface-900 dark:text-white">Create account</h1>
          <p class="mb-6 text-sm text-surface-500 dark:text-white/55">
            Start with email and password
          </p>

          <BaseInput
            v-model="email"
            id="register-email"
            label="Email"
            type="email"
            autocomplete="email"
            required
            class="w-full"
            :error="emailError"
            @blur="emit('validate', 'email')"
          />
          <BaseInput
            v-model="password"
            id="register-password"
            label="Password"
            type="password"
            autocomplete="new-password"
            required
            class="w-full"
            :error="passwordError"
            @blur="emit('validate', 'password')"
          />
          <Button
            type="submit"
            class="mt-4 min-w-36 justify-center font-semibold uppercase tracking-wider rounded-full py-2 bg-linear-to-br from-reisa-pink-500 to-reisa-lilac-500 hover:from-reisa-pink-400 hover:to-reisa-lilac-400 border-none text-white transition-all duration-200"
            :disabled="loading"
            :loading="loading"
          >
            Sign up
          </Button>
        </form>
      </div>

      <div
        class="absolute left-1/2 top-0 hidden h-full w-1/2 overflow-hidden transition-transform duration-700 ease-in-out md:block z-40"
        :class="{ '-translate-x-full': mode === 'Register' }"
      >
        <div
          class="relative -left-full flex h-full w-[200%] flex-row bg-reisa-lilac-50 dark:bg-surface-900 text-reisa-lilac-900 dark:text-white transition-transform duration-700 ease-in-out"
          :class="{ 'translate-x-1/2': mode === 'Register' }"
        >
          <div
            class="flex h-full w-1/2 flex-col items-center justify-center px-10 text-center"
          >
            <div class="flex flex-col items-center gap-2 mb-8">
              <span class="text-xs font-extrabold tracking-widest uppercase text-reisa-lilac-600/70 dark:text-white/50">Welcome Back</span>
              <h2 class="text-3xl font-extrabold tracking-tight text-reisa-lilac-950 dark:text-white leading-tight">
                Already have<br />an account?
              </h2>
            </div>
            <Button
              variant="outlined"
              class="border border-reisa-lilac-300 dark:border-white/30 text-reisa-lilac-700 dark:text-white hover:bg-reisa-lilac-600 dark:hover:bg-white hover:text-white dark:hover:text-surface-900 rounded-full font-bold uppercase tracking-wider py-2.5 px-7 transition-all duration-300 text-xs shadow-sm hover:shadow-md"
              :disabled="loading"
              @click="toggleMode"
              label="Go to log in"
            />
          </div>

          <div
            class="flex h-full w-1/2 flex-col items-center justify-center px-10 text-center"
          >
            <div class="flex flex-col items-center gap-2 mb-8">
              <span class="text-xs font-extrabold tracking-widest uppercase text-reisa-lilac-600/70 dark:text-white/50">Start Trading</span>
              <h2 class="text-3xl font-extrabold tracking-tight text-reisa-lilac-950 dark:text-white leading-tight">
                New to<br />QuantAura?
              </h2>
            </div>
            <Button
              variant="outlined"
              class="border border-reisa-lilac-300 dark:border-white/30 text-reisa-lilac-700 dark:text-white hover:bg-reisa-lilac-600 dark:hover:bg-white hover:text-white dark:hover:text-surface-900 rounded-full font-bold uppercase tracking-wider py-2.5 px-7 transition-all duration-300 text-xs shadow-sm hover:shadow-md"
              :disabled="loading"
              @click="toggleMode"
              label="Go to sign up"
            />
          </div>
        </div>
      </div>

      <div
        class="flex min-h-155 flex-col justify-center bg-surface-0 dark:bg-[oklch(0.18_0.018_285)] px-6 py-10 text-surface-900 dark:text-white md:hidden transition-colors duration-300"
      >
        <div class="mb-8 text-center">
          <div
            class="mb-4 inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-linear-to-br from-reisa-pink-500 to-reisa-lilac-500 shadow-lg shadow-reisa-pink-500/20"
          >
            <span class="text-2xl font-black text-white">A</span>
          </div>
          <h1 class="text-3xl font-black text-surface-900 dark:text-white">
            {{ mode === "Register" ? "Create account" : "Welcome back" }}
          </h1>
          <p class="mt-2 text-sm text-surface-500 dark:text-white/55">
            {{
              mode === "Register"
                ? "Start with email and password"
                : "Sign in to QuantAura"
            }}
          </p>
        </div>

        <form
          class="flex flex-col"
          @submit.prevent="submitAs(mode === 'Register' ? 'Register' : 'Login')"
        >
          <BaseInput
            v-model="email"
            id="mobile-email"
            label="Email"
            type="email"
            autocomplete="email"
            required
            class="w-full"
            :error="emailError"
            @blur="emit('validate', 'email')"
          />
          <BaseInput
            v-model="password"
            id="mobile-password"
            label="Password"
            type="password"
            :autocomplete="
              mode === 'Register' ? 'new-password' : 'current-password'
            "
            required
            class="w-full"
            :error="passwordError"
            @blur="emit('validate', 'password')"
          />
          <Button
            type="submit"
            class="mt-2 w-full justify-center font-semibold uppercase tracking-wider rounded-full py-2 bg-linear-to-br from-reisa-pink-500 to-reisa-lilac-500 hover:from-reisa-pink-400 hover:to-reisa-lilac-400 border-none text-white transition-all duration-200"
            :disabled="loading"
            :loading="loading"
          >
            {{ mode === "Register" ? "Create account" : "Log in" }}
          </Button>
        </form>

        <Button
          variant="outlined"
          class="mt-4 w-full justify-center border-surface-300 dark:border-white/20 text-surface-700 dark:text-white hover:bg-surface-100 dark:hover:bg-white/10 rounded-full py-2"
          :disabled="loading"
          @click="toggleMode"
          :label="mode === 'Register' ? 'Go to log in' : 'Go to sign up'"
        />
      </div>
    </div>
  </div>
</template>
