from fastapi import APIRouter, Depends, HTTPException

from ..core.db import get_db
from ..core.security import create_access_token, verify_password
from ..models import User
from ..schemas import LoginIn, TokenOut

router = APIRouter()


@router.post("/auth/login", response_model=TokenOut)
def login(body: LoginIn, db=Depends(get_db)) -> TokenOut:
    user = db.query(User).filter_by(email=body.email).first()
    if user is None or not verify_password(body.password, user.password_hash):
        raise HTTPException(status_code=401)
    return TokenOut(access_token=create_access_token(user.id))
