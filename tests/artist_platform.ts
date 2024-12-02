import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ArtistPlatform } from "../target/types/artist_platform";
import { expect } from "chai";
import { PublicKey, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("artist_platform", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ArtistPlatform as Program<ArtistPlatform>;
  
  // Test wallets
  const artist = anchor.web3.Keypair.generate();
  const follower = anchor.web3.Keypair.generate();
  const collaborator = anchor.web3.Keypair.generate();
  
  // PDAs
  let artistProfilePda: PublicKey;
  let tipsVaultPda: PublicKey;
  let followerAccountPda: PublicKey;
  let workPda: PublicKey;
  let interactionPda: PublicKey;
  let collabRequestPda: PublicKey;

  before(async () => {
    // Airdrop SOL to test wallets
    const signatures = await Promise.all([
      provider.connection.requestAirdrop(artist.publicKey, 10 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(follower.publicKey, 10 * LAMPORTS_PER_SOL),
      provider.connection.requestAirdrop(collaborator.publicKey, 10 * LAMPORTS_PER_SOL),
    ]);

    await Promise.all(signatures.map(sig => provider.connection.confirmTransaction(sig)));

    // Find PDAs
    [artistProfilePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("artist_profile"), artist.publicKey.toBuffer()],
      program.programId
    );

    [tipsVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("tips_vault"), artistProfilePda.toBuffer()],
      program.programId
    );
  });

  describe("Happy Path Scenarios", () => {
    it("Creates an artist profile", async () => {
      await program.methods
        .createArtistProfile(
          "Test Artist",
          "Digital artist creating awesome NFTs",
          ["https://twitter.com/testartist"]
        )
        .accounts({
          artistProfile: artistProfilePda,
          owner: artist.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([artist])
        .rpc();

      const profile = await program.account.artistProfile.fetch(artistProfilePda);
      expect(profile.owner.toString()).to.equal(artist.publicKey.toString());
      expect(profile.name).to.equal("Test Artist");
      expect(profile.followerCount.toNumber()).to.equal(0);
    });

    it("Updates artist profile", async () => {
      const newBio = "Updated artist bio";
      await program.methods
        .updateArtistProfile(
          null, // keep existing name
          newBio,
          null // keep existing links
        )
        .accounts({
          artistProfile: artistProfilePda,
          owner: artist.publicKey,
        })
        .signers([artist])
        .rpc();

      const profile = await program.account.artistProfile.fetch(artistProfilePda);
      expect(profile.bio).to.equal(newBio);
    });

    it("Follows an artist", async () => {
      [followerAccountPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("follower"),
          artistProfilePda.toBuffer(),
          follower.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .followArtist()
        .accounts({
          followerAccount: followerAccountPda,
          artistProfile: artistProfilePda,
          follower: follower.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([follower])
        .rpc();

      const followerAccount = await program.account.followerAccount.fetch(followerAccountPda);
      expect(followerAccount.isFollowing).to.be.true;

      const profile = await program.account.artistProfile.fetch(artistProfilePda);
      expect(profile.followerCount.toNumber()).to.equal(1);
    });

    it("Posts a new work", async () => {
      [workPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("work"),
          artistProfilePda.toBuffer(),
          Buffer.from([0]), // first work
        ],
        program.programId
      );

      await program.methods
        .postWork(
          "My First NFT",
          "A beautiful digital artwork",
          "https://arweave.net/artwork-hash"
        )
        .accounts({
          work: workPda,
          artistProfile: artistProfilePda,
          owner: artist.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([artist])
        .rpc();

      const work = await program.account.work.fetch(workPda);
      expect(work.title).to.equal("My First NFT");
      expect(work.likes.toNumber()).to.equal(0);
    });

    it("Tips an artist", async () => {
      const tipAmount = new anchor.BN(1 * LAMPORTS_PER_SOL);
      const initialBalance = await provider.connection.getBalance(tipsVaultPda);

      await program.methods
        .tipArtist(tipAmount)
        .accounts({
          artistProfile: artistProfilePda,
          tipsVault: tipsVaultPda,
          tipper: follower.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([follower])
        .rpc();

      const finalBalance = await provider.connection.getBalance(tipsVaultPda);
      expect(finalBalance - initialBalance).to.equal(tipAmount.toNumber());

      const profile = await program.account.artistProfile.fetch(artistProfilePda);
      expect(profile.totalTips.toNumber()).to.equal(tipAmount.toNumber());
    });

    it("Interacts with a work (like and comment)", async () => {
      // Like the work
      [interactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("interaction"),
          workPda.toBuffer(),
          follower.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .interactWithWork({ like: {} }, null)
        .accounts({
          work: workPda,
          interaction: interactionPda,
          user: follower.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([follower])
        .rpc();

      let work = await program.account.work.fetch(workPda);
      expect(work.likes.toNumber()).to.equal(1);

      // Comment on the work
      [interactionPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("interaction"),
          workPda.toBuffer(),
          collaborator.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .interactWithWork({ comment: {} }, "Great artwork!")
        .accounts({
          work: workPda,
          interaction: interactionPda,
          user: collaborator.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([collaborator])
        .rpc();

      work = await program.account.work.fetch(workPda);
      expect(work.commentCount.toNumber()).to.equal(1);
    });

    it("Creates and handles collaboration requests", async () => {
      [collabRequestPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("collab_request"),
          artistProfilePda.toBuffer(),
          collaborator.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .createCollabRequest("Let's create something together!")
        .accounts({
          collabRequest: collabRequestPda,
          artistProfile: artistProfilePda,
          requester: collaborator.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([collaborator])
        .rpc();

      let request = await program.account.collabRequest.fetch(collabRequestPda);
      expect(request.status).to.deep.equal({ pending: {} });

      // Accept the collaboration request
      await program.methods
        .updateCollabStatus({ accepted: {} })
        .accounts({
          collabRequest: collabRequestPda,
          artistProfile: artistProfilePda,
          owner: artist.publicKey,
        })
        .signers([artist])
        .rpc();

      request = await program.account.collabRequest.fetch(collabRequestPda);
      expect(request.status).to.deep.equal({ accepted: {} });
    });

    it("Withdraws tips", async () => {
      // First ensure there are tips to withdraw
      const tipAmount = new anchor.BN(1 * LAMPORTS_PER_SOL);
      
      // Add some tips first
      await program.methods
        .tipArtist(tipAmount)
        .accounts({
          artistProfile: artistProfilePda,
          tipsVault: tipsVaultPda,
          tipper: follower.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([follower])
        .rpc();

      // Now try to withdraw half of the tips
      const withdrawAmount = new anchor.BN(0.5 * LAMPORTS_PER_SOL);
      const initialBalance = await provider.connection.getBalance(artist.publicKey);

      await program.methods
        .withdrawTips(withdrawAmount)
        .accounts({
          artistProfile: artistProfilePda,
          tipsVault: tipsVaultPda,
          owner: artist.publicKey,
          artist: artist.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([artist])
        .rpc();

      const finalBalance = await provider.connection.getBalance(artist.publicKey);
      expect(finalBalance - initialBalance).to.be.approximately(
        withdrawAmount.toNumber(),
        1000000 // Allow for transaction fees
      );
    });
  });

  describe("Unhappy Path Scenarios", () => {
    it("Fails to create profile with too long name", async () => {
      const longName = "a".repeat(51);
      const newArtist = anchor.web3.Keypair.generate();
      
      // Airdrop some SOL
      const signature = await provider.connection.requestAirdrop(
        newArtist.publicKey,
        LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(signature);

      const [newArtistPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("artist_profile"), newArtist.publicKey.toBuffer()],
        program.programId
      );

      try {
        await program.methods
          .createArtistProfile(longName, "Bio", [])
          .accounts({
            artistProfile: newArtistPda,
            owner: newArtist.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([newArtist])
          .rpc();
        expect.fail("Should have failed with name too long error");
      } catch (error: any) {
        expect(error.toString()).to.include("Name is too long");
      }
    });

    it("Fails to follow artist twice", async () => {
      try {
        await program.methods
          .followArtist()
          .accounts({
            followerAccount: followerAccountPda,
            artistProfile: artistProfilePda,
            follower: follower.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([follower])
          .rpc();
        expect.fail("Should have failed with AlreadyFollowing error");
      } catch (error: any) {
        expect(error.toString()).to.include("Error: Simulation failed");
      }
    });

    it("Fails to tip with insufficient funds", async () => {
      const brokeUser = anchor.web3.Keypair.generate();
      try {
        await program.methods
          .tipArtist(new anchor.BN(LAMPORTS_PER_SOL))
          .accounts({
            artistProfile: artistProfilePda,
            tipsVault: tipsVaultPda,
            tipper: brokeUser.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([brokeUser])
          .rpc();
        expect.fail("Should have failed due to insufficient funds");
      } catch (error: any) {
        expect(error.toString()).to.include("insufficient lamports");
      }
    });

    it("Fails to withdraw more tips than available", async () => {
      const tooMuch = new anchor.BN(100 * LAMPORTS_PER_SOL);
      try {
        await program.methods
          .withdrawTips(tooMuch)
          .accounts({
            artistProfile: artistProfilePda,
            tipsVault: tipsVaultPda,
            artist: artist.publicKey,
            owner: artist.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([artist])
          .rpc();
        expect.fail("Should have failed with insufficient funds error");
      } catch (error: any) {
        expect(error.toString()).to.include("Insufficient funds");
      }
    });

    it("Fails to update collab request if not the artist", async () => {
      try {
        await program.methods
          .updateCollabStatus({ accepted: {} })
          .accounts({
            collabRequest: collabRequestPda,
            artistProfile: artistProfilePda,
            owner: follower.publicKey,
          })
          .signers([follower])
          .rpc();
        expect.fail("Should have failed with ownership verification");
      } catch (error: any) {
        expect(error.toString()).to.include("constraint was violated");
      }
    });

    it("Fails to like a work twice", async () => {
      try {
        await program.methods
          .interactWithWork({ like: {} }, null)
          .accounts({
            work: workPda,
            interaction: interactionPda,
            user: follower.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([follower])
          .rpc();
        expect.fail("Should have failed with Already Liked error");
      } catch (error: any) {
        expect(error.toString()).to.include("AnchorError");
      }
    });
  });
});