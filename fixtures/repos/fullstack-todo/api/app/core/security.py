from datetime import datetime, timedelta

from fastapi import Depends, HTTPException, Request
from jose import jwt
from passlib.context import CryptContext

SECRET = "change-me"
pwd_context = CryptContext(schemes=["bcrypt"])


def hash_password(raw: str) -> str:
    return pwd_context.hash(raw)


def verify_password(raw: str, hashed: str) -> bool:
    return pwd_context.verify(raw, hashed)


def create_access_token(user_id: int) -> str:
    exp = datetime.utcnow() + timedelta(hours=8)
    return jwt.encode({"sub": str(user_id), "exp": exp}, SECRET)


async def auth_middleware(request: Request):
    header = request.headers.get("Authorization", "")
    if not header.startswith("Bearer "):
        raise HTTPException(status_code=401)
    return jwt.decode(header[7:], SECRET)
