# Builds the app with Node, then serves the resulting static files with nginx.
FROM node:24-slim AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
# Vite bakes this into the bundle. "/api" is the same origin as the app: nginx forwards it to the server.
ENV VITE_API_BASE_URL=/api
# Vite first: it generates src/routeTree.gen.ts, which isn't in Git and which tsc needs.
RUN npx vite build && npx tsc -b

FROM nginx:1.28-alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
