FROM node:22-alpine
WORKDIR /web
COPY . .
RUN npm ci
EXPOSE 5173
CMD ["npm", "run", "dev", "--", "--host"]
