# --- STAGE 1: Build Stage ---
FROM node:22-slim AS builder

WORKDIR /webapp

# Copy package manifests first to leverage Docker layer caching
COPY package.json package-lock.json ./

# Install dependencies deterministically
RUN npm ci

# Copy the rest of your source code
COPY . .

# Build your frontend assets (e.g., Vite, Next.js static export, React build)
RUN npm run build


# --- STAGE 2: Production Runtime Stage ---
# Using nginxinc/nginx-unprivileged for secure, non-root container deployment
FROM nginxinc/nginx-unprivileged:1.27-alpine

# Copy the build output from Stage 1 into Nginx's default public web directory
COPY --from=builder /webapp/dist /usr/share/nginx/html

EXPOSE 3000

# The base image automatically starts nginx as a non-root user on port 3000
