use anchor_lang::prelude::*;

declare_id!("4r2r2VkCU1tXohrA3p71NXdwNgTG5NnV4WsNumFBzr1o");

#[program]
pub mod artist_platform {
    use super::*;

    // Create a new artist profile
    pub fn create_artist_profile(
        ctx: Context<CreateArtistProfile>,
        name: String,
        bio: String,
        links: Vec<String>,
    ) -> Result<()> {
        require!(name.len() <= 50, ArtistError::NameTooLong);
        require!(bio.len() <= 500, ArtistError::BioTooLong);
        require!(links.len() <= 5, ArtistError::TooManyLinks);

        let artist_profile = &mut ctx.accounts.artist_profile;
        artist_profile.owner = ctx.accounts.owner.key();
        artist_profile.name = name;
        artist_profile.bio = bio;
        artist_profile.links = links;
        artist_profile.follower_count = 0;
        artist_profile.total_tips = 0;
        artist_profile.work_count = 0;
        artist_profile.bump = ctx.bumps.artist_profile;

        Ok(())
    }

    // Create tips vault
    pub fn create_tips_vault(_ctx: Context<CreateTipsVault>) -> Result<()> {
        Ok(())
    }

    // Update artist profile
    pub fn update_artist_profile(
        ctx: Context<UpdateArtistProfile>,
        name: Option<String>,
        bio: Option<String>,
        links: Option<Vec<String>>,
    ) -> Result<()> {
        let artist_profile = &mut ctx.accounts.artist_profile;

        if let Some(new_name) = name {
            require!(new_name.len() <= 50, ArtistError::NameTooLong);
            artist_profile.name = new_name;
        }

        if let Some(new_bio) = bio {
            require!(new_bio.len() <= 500, ArtistError::BioTooLong);
            artist_profile.bio = new_bio;
        }

        if let Some(new_links) = links {
            require!(new_links.len() <= 5, ArtistError::TooManyLinks);
            artist_profile.links = new_links;
        }

        Ok(())
    }

    // Follow an artist
    pub fn follow_artist(ctx: Context<FollowArtist>) -> Result<()> {
        let follower_account = &mut ctx.accounts.follower_account;
        let artist_profile = &mut ctx.accounts.artist_profile;

        require!(
            !follower_account.is_following,
            ArtistError::AlreadyFollowing
        );

        follower_account.follower = ctx.accounts.follower.key();
        follower_account.artist = artist_profile.key();
        follower_account.is_following = true;
        follower_account.bump = ctx.bumps.follower_account;

        artist_profile.follower_count = artist_profile.follower_count.checked_add(1)
            .ok_or(ArtistError::NumericalOverflow)?;

        Ok(())
    }

    // Post new work
    pub fn post_work(
        ctx: Context<PostWork>,
        title: String,
        description: String,
        content_url: String,
    ) -> Result<()> {
        require!(title.len() <= 100, ArtistError::TitleTooLong);
        require!(description.len() <= 1000, ArtistError::DescriptionTooLong);
        
        let work = &mut ctx.accounts.work;
        let artist_profile = &mut ctx.accounts.artist_profile;

        work.artist = artist_profile.key();
        work.title = title;
        work.description = description;
        work.content_url = content_url;
        work.likes = 0;
        work.comment_count = 0;
        work.timestamp = Clock::get()?.unix_timestamp;
        work.bump = ctx.bumps.work;

        artist_profile.work_count = artist_profile.work_count.checked_add(1)
            .ok_or(ArtistError::NumericalOverflow)?;

        Ok(())
    }

    // Tip an artist
    pub fn tip_artist(ctx: Context<TipArtist>, amount: u64) -> Result<()> {
        require!(amount > 0, ArtistError::InvalidAmount);

        let artist_profile = &mut ctx.accounts.artist_profile;
        let tipper = &ctx.accounts.tipper;
        let tips_vault = &ctx.accounts.tips_vault;

        // Transfer SOL from tipper to tips vault
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: tipper.to_account_info(),
                to: tips_vault.to_account_info(),
            },
        );

        anchor_lang::system_program::transfer(cpi_context, amount)?;

        artist_profile.total_tips = artist_profile.total_tips.checked_add(amount)
            .ok_or(ArtistError::NumericalOverflow)?;

        Ok(())
    }

    // Interact with work (like/comment)
    pub fn interact_with_work(
        ctx: Context<InteractWithWork>,
        interaction_type: InteractionType,
        comment_text: Option<String>,
    ) -> Result<()> {
        let work = &mut ctx.accounts.work;
        let interaction = &mut ctx.accounts.interaction;

        match interaction_type {
            InteractionType::Like => {
                require!(!interaction.has_liked, ArtistError::AlreadyLiked);
                interaction.has_liked = true;
                work.likes = work.likes.checked_add(1)
                    .ok_or(ArtistError::NumericalOverflow)?;
            }
            InteractionType::Comment => {
                let comment = comment_text.ok_or(ArtistError::CommentRequired)?;
                require!(comment.len() <= 500, ArtistError::CommentTooLong);
                
                interaction.comment = Some(comment);
                work.comment_count = work.comment_count.checked_add(1)
                    .ok_or(ArtistError::NumericalOverflow)?;
            }
        }

        interaction.user = ctx.accounts.user.key();
        interaction.work = work.key();
        interaction.timestamp = Clock::get()?.unix_timestamp;
        interaction.bump = ctx.bumps.interaction;

        Ok(())
    }

    // Create collaboration request
    pub fn create_collab_request(
        ctx: Context<CreateCollabRequest>,
        description: String,
    ) -> Result<()> {
        let collab_request = &mut ctx.accounts.collab_request;
        
        collab_request.requester = ctx.accounts.requester.key();
        collab_request.artist = ctx.accounts.artist_profile.key();
        collab_request.description = description;
        collab_request.status = CollabStatus::Pending;
        collab_request.timestamp = Clock::get()?.unix_timestamp;
        collab_request.bump = ctx.bumps.collab_request;

        Ok(())
    }

    // Update collaboration request status
    pub fn update_collab_status(
        ctx: Context<UpdateCollabStatus>,
        status: CollabStatus,
    ) -> Result<()> {
        let collab_request = &mut ctx.accounts.collab_request;
        
        require!(
            collab_request.status == CollabStatus::Pending,
            ArtistError::CollabAlreadyResolved
        );

        collab_request.status = status;
        Ok(())
    }

    // Withdraw tips
    pub fn withdraw_tips(ctx: Context<WithdrawTips>, amount: u64) -> Result<()> {
        let tips_vault = &ctx.accounts.tips_vault;
        let artist = &ctx.accounts.artist;
    
        let vault_balance = tips_vault.lamports();
        require!(vault_balance >= amount, ArtistError::InsufficientFunds);
    
        // Keep minimum rent balance
        let rent = Rent::get()?;
        let minimum_balance = rent.minimum_balance(0);
        require!(
            vault_balance.checked_sub(amount).unwrap() >= minimum_balance,
            ArtistError::InsufficientFunds
        );
    
        // Transfer lamports from vault to artist
        **tips_vault.lamports.borrow_mut() = vault_balance.checked_sub(amount).unwrap();
        **artist.lamports.borrow_mut() = artist.lamports().checked_add(amount).unwrap();
    
        Ok(())
    }

    // Close artist profile
    pub fn close_artist_profile(ctx: Context<CloseArtistProfile>) -> Result<()> {
        // Transfer remaining lamports to the artist
        let tips_vault = &ctx.accounts.tips_vault;
        let remaining_balance = tips_vault.lamports();

        if remaining_balance > 0 {
            **tips_vault.try_borrow_mut_lamports()? = 0;
            **ctx.accounts.artist.try_borrow_mut_lamports()? = ctx
                .accounts.artist
                .lamports()
                .checked_add(remaining_balance)
                .ok_or(ArtistError::NumericalOverflow)?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateArtistProfile<'info> {
    #[account(
        init,
        payer = owner,
        space = ArtistProfile::LEN,
        seeds = [b"artist_profile", owner.key().as_ref()],
        bump
    )]
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateTipsVault<'info> {
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(
        init,
        payer = owner,
        space = 0,
        seeds = [b"tips_vault", artist_profile.key().as_ref()],
        bump
    )]
    pub tips_vault: SystemAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateArtistProfile<'info> {
    #[account(
        mut,
        seeds = [b"artist_profile", owner.key().as_ref()],
        bump = artist_profile.bump,
        has_one = owner
    )]
    pub artist_profile: Account<'info, ArtistProfile>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct FollowArtist<'info> {
    #[account(
        init,
        payer = follower,
        space = FollowerAccount::LEN,
        seeds = [
            b"follower",
            artist_profile.key().as_ref(),
            follower.key().as_ref()
        ],
        bump
    )]
    pub follower_account: Account<'info, FollowerAccount>,
    #[account(mut)]
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(mut)]
    pub follower: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PostWork<'info> {
    #[account(
        init,
        payer = owner,
        space = Work::LEN,
        seeds = [
            b"work",
            artist_profile.key().as_ref(),
            &[artist_profile.work_count]
        ],
        bump
    )]
    pub work: Account<'info, Work>,
    #[account(
        mut,
        seeds = [b"artist_profile", owner.key().as_ref()],
        bump = artist_profile.bump,
        has_one = owner
    )]
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TipArtist<'info> {
    #[account(mut)]
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(
        mut,
        seeds = [b"tips_vault", artist_profile.key().as_ref()],
        bump
    )]
    pub tips_vault: SystemAccount<'info>,
    #[account(mut)]
    pub tipper: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InteractWithWork<'info> {
    #[account(mut)]
    pub work: Account<'info, Work>,
    #[account(
        init,
        payer = user,
        space = Interaction::LEN,
        seeds = [
            b"interaction",
            work.key().as_ref(),
            user.key().as_ref()
        ],
        bump
    )]
    pub interaction: Account<'info, Interaction>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateCollabRequest<'info> {
    #[account(
        init,
        payer = requester,
        space = CollabRequest::LEN,
        seeds = [
            b"collab_request",
            artist_profile.key().as_ref(),
            requester.key().as_ref()
        ],
        bump
    )]
    pub collab_request: Account<'info, CollabRequest>,
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(mut)]
    pub requester: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCollabStatus<'info> {
    #[account(
        mut,
        seeds = [
            b"collab_request",
            artist_profile.key().as_ref(),
            collab_request.requester.as_ref()
        ],
        bump = collab_request.bump
    )]
    pub collab_request: Account<'info, CollabRequest>,
    #[account(
        seeds = [b"artist_profile", owner.key().as_ref()],
        bump = artist_profile.bump,
        has_one = owner
    )]
    pub artist_profile: Account<'info, ArtistProfile>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawTips<'info> {
    #[account(
        has_one = owner,
        seeds = [b"artist_profile", owner.key().as_ref()],
        bump = artist_profile.bump
    )]
    pub artist_profile: Account<'info, ArtistProfile>,
    
    #[account(
        mut,
        seeds = [b"tips_vault", artist_profile.key().as_ref()],
        bump,
    )]
    pub tips_vault: SystemAccount<'info>,
    
    #[account(mut)]
    pub artist: SystemAccount<'info>,
    
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseArtistProfile<'info> {
    #[account(
        mut,
        close = artist,
        seeds = [b"artist_profile", artist.key().as_ref()],
        bump = artist_profile.bump,
        has_one = owner
    )]
    pub artist_profile: Account<'info, ArtistProfile>,
    #[account(
        mut,
        seeds = [b"tips_vault", artist_profile.key().as_ref()],
        bump
    )]
    pub tips_vault: SystemAccount<'info>,
    #[account(mut)]
    pub artist: SystemAccount<'info>,
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct ArtistProfile {
    pub owner: Pubkey,
    pub name: String,
    pub bio: String,
    pub links: Vec<String>,
    pub follower_count: u64,
    pub total_tips: u64,
    pub work_count: u8,
    pub bump: u8,
}

#[account]
pub struct FollowerAccount {
    pub follower: Pubkey,
    pub artist: Pubkey,
    pub is_following: bool,
    pub bump: u8,
}

#[account]
pub struct Work {
    pub artist: Pubkey,
    pub title: String,
    pub description: String,
    pub content_url: String,
    pub likes: u64,
    pub comment_count: u64,
    pub timestamp: i64,
    pub bump: u8,
}

#[account]
pub struct Interaction {
    pub user: Pubkey,
    pub work: Pubkey,
    pub has_liked: bool,
    pub comment: Option<String>,
    pub timestamp: i64,
    pub bump: u8,
}

#[account]
pub struct CollabRequest {
    pub requester: Pubkey,
    pub artist: Pubkey,
    pub description: String,
    pub status: CollabStatus,
    pub timestamp: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum CollabStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum InteractionType {
    Like,
    Comment,
}

// Error codes
#[error_code]
pub enum ArtistError {
    #[msg("Name is too long")]
    NameTooLong,
    #[msg("Bio is too long")]
    BioTooLong,
    #[msg("Too many links")]
    TooManyLinks,
    #[msg("Already following this artist")]
    AlreadyFollowing,
    #[msg("Title is too long")]
    TitleTooLong,
    #[msg("Description is too long")]
    DescriptionTooLong,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Already liked this work")]
    AlreadyLiked,
    #[msg("Comment is required")]
    CommentRequired,
    #[msg("Comment is too long")]
    CommentTooLong,
    #[msg("Collaboration request already resolved")]
    CollabAlreadyResolved,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Numerical overflow")]
    NumericalOverflow,
}

// Constants for account sizes
impl ArtistProfile {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner pubkey
        4 + 50 + // name: String (size + max length)
        4 + 500 + // bio: String (size + max length)
        4 + (4 + 100) * 5 + // links: Vec<String> (vector size + (string size + max length) * max links)
        8 + // follower_count: u64
        8 + // total_tips: u64
        1 + // work_count: u8
        1 + // bump
        200; // padding for future updates
}

impl FollowerAccount {
    pub const LEN: usize = 8 + // discriminator
        32 + // follower pubkey
        32 + // artist pubkey
        1 + // is_following: bool
        1 + // bump
        64; // padding
}

impl Work {
    pub const LEN: usize = 8 + // discriminator
        32 + // artist pubkey
        4 + 100 + // title: String (size + max length)
        4 + 1000 + // description: String (size + max length)
        4 + 200 + // content_url: String (size + max length)
        8 + // likes: u64
        8 + // comment_count: u64
        8 + // timestamp: i64
        1 + // bump
        200; // padding
}

impl Interaction {
    pub const LEN: usize = 8 + // discriminator
        32 + // user pubkey
        32 + // work pubkey
        1 + // has_liked: bool
        (1 + 4 + 500) + // Option<String> comment (discriminator + size + max length)
        8 + // timestamp: i64
        1 + // bump
        100; // padding
}

impl CollabRequest {
    pub const LEN: usize = 8 + // discriminator
        32 + // requester pubkey
        32 + // artist pubkey
        4 + 500 + // description: String (size + max length)
        1 + // status: CollabStatus
        8 + // timestamp: i64
        1 + // bump
        100; // padding
}