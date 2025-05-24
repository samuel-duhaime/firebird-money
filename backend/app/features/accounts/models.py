from enum import Enum
from typing import Optional
from pydantic import EmailStr
from sqlmodel import SQLModel, Field


class AccountStatus(str, Enum):
    verified = "verified"
    pending = "pending"
    suspended = "suspended"


class Account(SQLModel, table=True):
    __tablename__ = "accounts"
    id: Optional[int] = Field(default=None, primary_key=True)
    google_id: Optional[str] = None
    status: AccountStatus
    email: EmailStr


class AccountCreate(SQLModel):
    email: EmailStr
    google_id: Optional[str] = None


class AccountPatch(SQLModel):
    email: Optional[EmailStr] = None
    google_id: Optional[str] = None
    status: Optional[AccountStatus] = None
