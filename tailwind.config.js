/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{vue,js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: {
          DEFAULT: "rgb(var(--color-bg) / <alpha-value>)",
          deep: "rgb(var(--color-bg-deep) / <alpha-value>)",
          card: "rgb(var(--color-bg-card) / <alpha-value>)",
          hover: "rgb(var(--color-bg-hover) / <alpha-value>)",
          border: "rgb(var(--color-bg-border) / <alpha-value>)",
        },
        accent: {
          DEFAULT: "rgb(var(--color-accent) / <alpha-value>)",
          light: "rgb(var(--color-accent-light) / <alpha-value>)",
          dark: "rgb(var(--color-accent-dark) / <alpha-value>)",
        },
        deep: {
          DEFAULT: "#0f3460",
          light: "#1a4a86",
        },
        muted: "rgb(var(--color-text-muted) / <alpha-value>)",
        white: "rgb(var(--color-text-main) / <alpha-value>)",
        "pure-white": "#ffffff",
        "pure-black": "#000000",
      },
      fontFamily: {
        sans: [
          "Segoe UI",
          "Microsoft YaHei UI",
          "Microsoft YaHei",
          "PingFang SC",
          "sans-serif",
        ],
      },
      animation: {
        "fade-in": "fadeIn 0.2s ease-out",
        "slide-up": "slideUp 0.25s ease-out",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        slideUp: {
          "0%": { transform: "translateY(12px)", opacity: "0" },
          "100%": { transform: "translateY(0)", opacity: "1" },
        },
      },
    },
  },
  plugins: [],
};
