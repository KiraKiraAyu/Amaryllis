import { describe, it, expect, beforeEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useAuthStore } from "@/stores/auth";

// Mock router
vi.mock("@/router", () => ({
  default: {
    push: vi.fn(),
  },
}));

// Mock API
vi.mock("@/api/auth", () => ({
  getAuthStatusApi: vi.fn().mockResolvedValue({ configured: true }),
  verifyApi: vi.fn().mockResolvedValue({ token: "verified-token", message: "ok" }),
  setupStartApi: vi.fn(),
  setupConfirmApi: vi.fn(),
  resetStartApi: vi.fn(),
  resetConfirmApi: vi.fn(),
}));

describe("Auth Store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("should initialize with default state", () => {
    const store = useAuthStore();
    expect(store.isLoggedIn).toBe(false);
    expect(store.token).toBe("");
    expect(store.configured).toBeNull();
  });

  it("should restore token from localStorage", () => {
    localStorage.setItem("quantaura_token", "saved-token");
    const store = useAuthStore();
    expect(store.isLoggedIn).toBe(true);
    expect(store.token).toBe("saved-token");
  });

  it("should set token on setToken", () => {
    const store = useAuthStore();

    store.setToken("test-token");

    expect(store.isLoggedIn).toBe(true);
    expect(store.token).toBe("test-token");
    expect(localStorage.getItem("quantaura_token")).toBe("test-token");
  });

  it("should set token after successful verify", async () => {
    const store = useAuthStore();

    await store.verify("123456");

    expect(store.isLoggedIn).toBe(true);
    expect(store.token).toBe("verified-token");
    expect(localStorage.getItem("quantaura_token")).toBe("verified-token");
  });

  it("should refresh configured status from the server", async () => {
    const store = useAuthStore();

    const configured = await store.refreshStatus();

    expect(configured).toBe(true);
    expect(store.configured).toBe(true);
  });

  it("should clear token on logout", () => {
    const store = useAuthStore();
    store.setToken("test-token");

    store.logout();

    expect(store.isLoggedIn).toBe(false);
    expect(store.token).toBe("");
    expect(localStorage.getItem("quantaura_token")).toBeNull();
  });
});
