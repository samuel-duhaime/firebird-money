from enum import Enum
from typing import Optional
from sqlmodel import SQLModel, Field


class UserType(str, Enum):
    family_manager = "family_manager"
    family_member = "family_member"
    financial_professional = "financial_professional"


class User(SQLModel, table=True):
    __tablename__ = "users"
    id: Optional[int] = Field(default=None, primary_key=True)
    account_id: int
    first_name: Optional[str] = None
    last_name: Optional[str] = None
    type: UserType
    logo_url: Optional[str] = None


class UserCreate(SQLModel):
    account_id: int
    type: UserType
    first_name: Optional[str] = None
    last_name: Optional[str] = None
    logo_url: Optional[str] = None


class UserPatch(SQLModel):
    first_name: Optional[str] = None
    last_name: Optional[str] = None
    type: Optional[UserType] = None
    logo_url: Optional[str] = None
