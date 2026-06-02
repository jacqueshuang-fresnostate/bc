/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        ink: '#182230',
        line: '#d8dee8',
        panel: '#f7f9fc',
        accent: '#0f766e',
        warning: '#b45309',
      },
    },
  },
  plugins: [],
};
