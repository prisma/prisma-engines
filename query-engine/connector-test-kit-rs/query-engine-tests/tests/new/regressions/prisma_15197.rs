use query_engine_tests::*;

#[test_suite(schema(schema), only(mongodb))]
mod prisma_14703 {
    fn schema() -> String {
        String::from(indoc! {r#"
        model Team {
          id        String    @id @default(auto()) @map("_id") @test.ObjectId
          createdAt DateTime? @default(now())
          updatedAt DateTime? @updatedAt
          teamId    Int
          leagueId  Int
          league    League    @relation(fields: [leagueId], references: [leagueId])
        
          abbrName       String
          cityName       String
          defScheme      Int
          displayName    String
          divName        String
          injuryCount    Int
          logoId         Int
          nickName       String
          offScheme      Int
          ovrRating      Int
          primaryColor   Int
          secondaryColor Int
          userName       String
        
          standings         Standing[]
        
          @@unique([leagueId, teamId])
        }
        
        model League {
          id        String    @id @default(auto()) @map("_id") @test.ObjectId
          createdAt DateTime? @default(now())
          updatedAt DateTime? @updatedAt
          leagueId  Int       @unique
          name      String
          slug      String
          discordId String?
          console   String?
          year      Int?

          standings Standing[]
          teams     Team[]
        }
        
        
            model Standing {
              id       String @id @default(auto()) @map("_id") @test.ObjectId
              leagueId Int
              league   League @relation(fields: [leagueId], references: [leagueId])
              teamId   Int
              team     Team   @relation(fields: [teamId, leagueId], references: [teamId, leagueId])      
              awayLosses      Int?
              awayTies        Int?
              awayWins        Int?
              calendarYear    Int?
              capAvailable    Int?
              capRoom         Int?
              capSpent        Int?
              confLosses      Int?
              confTies        Int?
              confWins        Int?
              conferenceId    Int?
              conferenceName  String
              defPassYds      Int?
              defPassYdsRank  Int?
              defRushYds      Int?
              defRushYdsRank  Int?
              defTotalYds     Int?
              defTotalYdsRank Int?
              divLosses       Int?
              divTies         Int?
              divWins         Int?
              divisionId      Int?
              divisionName    String?
              homeLosses      Int?
              homeTies        Int?
              homeWins        Int?
              netPts          Int?
              offPassYds      Int?
              offPassYdsRank  Int?
              offRushYds      Int?
              offRushYdsRank  Int?
              offTotalYds     Int?
              offTotalYdsRank Int?
              playoffStatus   Int?
              prevRank        Int?
              ptsAgainst      Int?
              ptsAgainstRank  Int?
              ptsFor          Int?
              ptsForRank      Int?
              rank            Int?
              seasonIndex     Int?
              seed            Int?
              stageIndex      Int?
              tODiff          Int?
              teamName        String?
              teamOvr         Int?
              totalLosses     Int?
              totalTies       Int?
              totalWins       Int?
              weekIndex       Int?
              winLossStreak   Int?
              winPct          Float?
            
              @@unique([leagueId, teamId])
            }
        "#})
    }

    #[connector_test]
    async fn very_large_upsert(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
  upsertOneStanding(
    create: {
      awayLosses: 0
      awayTies: 0
      awayWins: 0
      calendarYear: 2022
      conferenceId: 958267392
      confLosses: 0
      conferenceName: "AFC"
      confTies: 0
      confWins: 0
      capRoom: 218200000
      capAvailable: 27850000
      capSpent: 190350000
      defPassYds: 0
      defPassYdsRank: 8
      defRushYds: 0
      defRushYdsRank: 8
      defTotalYds: 0
      defTotalYdsRank: 8
      divisionId: 969539585
      divLosses: 0
      divisionName: "AFC North"
      divTies: 0
      divWins: 0
      homeLosses: 0
      homeTies: 0
      homeWins: 0
      netPts: 0
      offPassYds: 0
      offPassYdsRank: 8
      offRushYds: 0
      offRushYdsRank: 8
      offTotalYds: 0
      offTotalYdsRank: 8
      ptsAgainstRank: 8
      ptsForRank: 8
      playoffStatus: 0
      prevRank: 0
      ptsAgainst: 0
      ptsFor: 0
      rank: 0
      seed: 0
      seasonIndex: 0
      stageIndex: 0
      totalLosses: 0
      totalTies: 0
      totalWins: 0
      teamId: 972030012
      teamName: "Ravens"
      teamOvr: 82
      tODiff: 0
      weekIndex: 0
      winLossStreak: 0
      winPct: 0
      leagueId: 2363725
    }
    update: {
      awayLosses: 0
      awayTies: 0
      awayWins: 0
      calendarYear: 2022
      conferenceId: 958267392
      confLosses: 0
      conferenceName: "AFC"
      confTies: 0
      confWins: 0
      capRoom: 218200000
      capAvailable: 27850000
      capSpent: 190350000
      defPassYds: 0
      defPassYdsRank: 8
      defRushYds: 0
      defRushYdsRank: 8
      defTotalYds: 0
      defTotalYdsRank: 8
      divisionId: 969539585
      divLosses: 0
      divisionName: "AFC North"
      divTies: 0
      divWins: 0
      homeLosses: 0
      homeTies: 0
      homeWins: 0
      netPts: 0
      offPassYds: 0
      offPassYdsRank: 8
      offRushYds: 0
      offRushYdsRank: 8
      offTotalYds: 0
      offTotalYdsRank: 8
      ptsAgainstRank: 8
      ptsForRank: 8
      playoffStatus: 0
      prevRank: 0
      ptsAgainst: 0
      ptsFor: 0
      rank: 0
      seed: 0
      seasonIndex: 0
      stageIndex: 0
      totalLosses: 0
      totalTies: 0
      totalWins: 0
      teamId: 972030012
      teamName: "Ravens"
      teamOvr: 82
      tODiff: 0
      weekIndex: 0
      winLossStreak: 0
      winPct: 0
      leagueId: 2363725
    }
    where: {
      leagueId_teamId: {
        leagueId: 2363725
        teamId: 972030012
      }
    }
  ) {
    id
    leagueId
    teamId
    awayLosses
    awayTies
    awayWins
    calendarYear
    capAvailable
    capRoom
    capSpent
    confLosses
    confTies
    confWins
    conferenceId
    conferenceName
    defPassYds
    defPassYdsRank
    defRushYds
    defRushYdsRank
    defTotalYds
    defTotalYdsRank
    divLosses
    divTies
    divWins
    divisionId
    divisionName
    homeLosses
    homeTies
    homeWins
    netPts
    offPassYds
    offPassYdsRank
    offRushYds
    offRushYdsRank
    offTotalYds
    offTotalYdsRank
    playoffStatus
    prevRank
    ptsAgainst
    ptsAgainstRank
    ptsFor
    ptsForRank
    rank
    seasonIndex
    seed
    stageIndex
    tODiff
    teamName
    teamOvr
    totalLosses
    totalTies
    totalWins
    weekIndex
    winLossStreak
    winPct
  }}"#
        );

        run_query!(
            &runner,
            r#"mutation {
  upsertOneStanding(
    create: {
      awayLosses: 0
      awayTies: 0
      awayWins: 0
      calendarYear: 2022
      conferenceId: 958267392
      confLosses: 0
      conferenceName: "AFC"
      confTies: 0
      confWins: 0
      capRoom: 218200000
      capAvailable: 27850000
      capSpent: 190350000
      defPassYds: 0
      defPassYdsRank: 8
      defRushYds: 0
      defRushYdsRank: 8
      defTotalYds: 0
      defTotalYdsRank: 8
      divisionId: 969539585
      divLosses: 0
      divisionName: "AFC North"
      divTies: 0
      divWins: 0
      homeLosses: 0
      homeTies: 0
      homeWins: 0
      netPts: 0
      offPassYds: 0
      offPassYdsRank: 8
      offRushYds: 0
      offRushYdsRank: 8
      offTotalYds: 0
      offTotalYdsRank: 8
      ptsAgainstRank: 8
      ptsForRank: 8
      playoffStatus: 0
      prevRank: 0
      ptsAgainst: 0
      ptsFor: 0
      rank: 0
      seed: 0
      seasonIndex: 0
      stageIndex: 0
      totalLosses: 0
      totalTies: 0
      totalWins: 0
      teamId: 972030012
      teamName: "Ravens"
      teamOvr: 82
      tODiff: 0
      weekIndex: 0
      winLossStreak: 0
      winPct: 0
      leagueId: 2363725
    }
    update: {
      awayLosses: 0
      awayTies: 0
      awayWins: 0
      calendarYear: 2022
      conferenceId: 958267392
      confLosses: 0
      conferenceName: "AFC"
      confTies: 0
      confWins: 0
      capRoom: 218200000
      capAvailable: 27850000
      capSpent: 190350000
      defPassYds: 0
      defPassYdsRank: 8
      defRushYds: 0
      defRushYdsRank: 8
      defTotalYds: 0
      defTotalYdsRank: 8
      divisionId: 969539585
      divLosses: 0
      divisionName: "AFC North"
      divTies: 0
      divWins: 0
      homeLosses: 0
      homeTies: 0
      homeWins: 0
      netPts: 0
      offPassYds: 0
      offPassYdsRank: 8
      offRushYds: 0
      offRushYdsRank: 8
      offTotalYds: 0
      offTotalYdsRank: 8
      ptsAgainstRank: 8
      ptsForRank: 8
      playoffStatus: 0
      prevRank: 0
      ptsAgainst: 0
      ptsFor: 0
      rank: 0
      seed: 0
      seasonIndex: 0
      stageIndex: 0
      totalLosses: 0
      totalTies: 0
      totalWins: 0
      teamId: 972030012
      teamName: "Ravens"
      teamOvr: 82
      tODiff: 0
      weekIndex: 0
      winLossStreak: 0
      winPct: 0
      leagueId: 2363725
    }
    where: {
      leagueId_teamId: {
        leagueId: 2363725
        teamId: 972030012
      }
    }
  ) {
    id
    leagueId
    teamId
    awayLosses
    awayTies
    awayWins
    calendarYear
    capAvailable
    capRoom
    capSpent
    confLosses
    confTies
    confWins
    conferenceId
    conferenceName
    defPassYds
    defPassYdsRank
    defRushYds
    defRushYdsRank
    defTotalYds
    defTotalYdsRank
    divLosses
    divTies
    divWins
    divisionId
    divisionName
    homeLosses
    homeTies
    homeWins
    netPts
    offPassYds
    offPassYdsRank
    offRushYds
    offRushYdsRank
    offTotalYds
    offTotalYdsRank
    playoffStatus
    prevRank
    ptsAgainst
    ptsAgainstRank
    ptsFor
    ptsForRank
    rank
    seasonIndex
    seed
    stageIndex
    tODiff
    teamName
    teamOvr
    totalLosses
    totalTies
    totalWins
    weekIndex
    winLossStreak
    winPct
  }}"#
        );

        run_query!(
            &runner,
            r#"mutation {
  upsertOneStanding(
    create: {
      awayLosses: 0
      awayTies: 0
      awayWins: 0
      calendarYear: 2022
      conferenceId: 958267392
      confLosses: 0
      conferenceName: "AFC"
      confTies: 0
      confWins: 0
      capRoom: 218200000
      capAvailable: 27850000
      capSpent: 190350000
      defPassYds: 0
      defPassYdsRank: 8
      defRushYds: 0
      defRushYdsRank: 8
      defTotalYds: 0
      defTotalYdsRank: 8
      divisionId: 969539585
      divLosses: 0
      divisionName: "AFC North"
      divTies: 0
      divWins: 0
      homeLosses: 0
      homeTies: 0
      homeWins: 0
      netPts: 0
      offPassYds: 0
      offPassYdsRank: 8
      offRushYds: 0
      offRushYdsRank: 8
      offTotalYds: 0
      offTotalYdsRank: 8
      ptsAgainstRank: 8
      ptsForRank: 8
      playoffStatus: 0
      prevRank: 0
      ptsAgainst: 0
      ptsFor: 0
      rank: 0
      seed: 0
      seasonIndex: 0
      stageIndex: 0
      totalLosses: 0
      totalTies: 0
      totalWins: 0
      teamId: 972030012
      teamName: "Ravens"
      teamOvr: 82
      tODiff: 0
      weekIndex: 0
      winLossStreak: 0
      winPct: 0
      leagueId: 2363725
    }
    update: {
      awayLosses: 0
      awayTies: 0
      awayWins: 0
      calendarYear: 2022
      conferenceId: 958267392
      confLosses: 0
      conferenceName: "AFC"
      confTies: 0
      confWins: 0
      capRoom: 218200000
      capAvailable: 27850000
      capSpent: 190350000
      defPassYds: 0
      defPassYdsRank: 8
      defRushYds: 0
      defRushYdsRank: 8
      defTotalYds: 0
      defTotalYdsRank: 8
      divisionId: 969539585
      divLosses: 0
      divisionName: "AFC North"
      divTies: 0
      divWins: 0
      homeLosses: 0
      homeTies: 0
      homeWins: 0
      netPts: 0
      offPassYds: 0
      offPassYdsRank: 8
      offRushYds: 0
      offRushYdsRank: 8
      offTotalYds: 0
      offTotalYdsRank: 8
      ptsAgainstRank: 8
      ptsForRank: 8
      playoffStatus: 0
      prevRank: 0
      ptsAgainst: 0
      ptsFor: 0
      rank: 0
      seed: 0
      seasonIndex: 0
      stageIndex: 0
      totalLosses: 0
      totalTies: 0
      totalWins: 0
      teamId: 972030012
      teamName: "Ravens"
      teamOvr: 82
      tODiff: 0
      weekIndex: 0
      winLossStreak: 0
      winPct: 0
      leagueId: 2363725
    }
    where: {
      leagueId_teamId: {
        leagueId: 2363725
        teamId: 972030012
      }
    }
  ) {
    id
    leagueId
    teamId
    awayLosses
    awayTies
    awayWins
    calendarYear
    capAvailable
    capRoom
    capSpent
    confLosses
    confTies
    confWins
    conferenceId
    conferenceName
    defPassYds
    defPassYdsRank
    defRushYds
    defRushYdsRank
    defTotalYds
    defTotalYdsRank
    divLosses
    divTies
    divWins
    divisionId
    divisionName
    homeLosses
    homeTies
    homeWins
    netPts
    offPassYds
    offPassYdsRank
    offRushYds
    offRushYdsRank
    offTotalYds
    offTotalYdsRank
    playoffStatus
    prevRank
    ptsAgainst
    ptsAgainstRank
    ptsFor
    ptsForRank
    rank
    seasonIndex
    seed
    stageIndex
    tODiff
    teamName
    teamOvr
    totalLosses
    totalTies
    totalWins
    weekIndex
    winLossStreak
    winPct
  }}"#
        );

        // assert!(false);
        Ok(())
    }
}
